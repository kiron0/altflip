use crate::pb::{self, Val};
use crate::{epoch_to_rfc3339, json, version_of, Ctx, Detection, OutputSink, Provider};
use altflip_core::process::{self, ProcView};
use altflip_core::resource::{Locator, Role, Snapshot};
use altflip_core::{
    jwt, short_hash, AuthHealth, Capabilities, Compatibility, Error, Identity, ProcessInfo, ProcessKind, ProviderId,
    Result, Support, SwitchMode, UsageReading,
};
use base64::Engine;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const SERVICE: &str = "gemini";
pub const ACCOUNT: &str = "antigravity";
const B64_PREFIX: &str = "go-keyring-base64:";
const KEY_OAUTH: &str = "antigravityUnifiedStateSync.oauthToken";
const KEY_STATUS: &str = "antigravityUnifiedStateSync.userStatus";
const KEY_CREDITS: &str = "antigravityUnifiedStateSync.modelCredits";
const KEY_AUTH_SYNC: &str = r#"secret://{"extensionId":"google.google-antigravity","key":"antigravity.authSync"}"#;
const APP_DIRS: [&str; 2] = ["Antigravity IDE", "Antigravity"];

pub struct Antigravity {
    ctx: Ctx,
}

fn decode_payload(raw: &[u8]) -> Option<serde_json::Value> {
    let s = std::str::from_utf8(raw).ok()?.trim();
    if let Some(b) = s.strip_prefix(B64_PREFIX) {
        let bytes = base64::engine::general_purpose::STANDARD.decode(b).ok()?;
        return serde_json::from_slice(&bytes).ok();
    }
    serde_json::from_str(s).ok()
}

fn payload_email(v: &serde_json::Value) -> Option<String> {
    v.get("id_token_email")
        .and_then(|e| e.as_str())
        .map(str::to_string)
        .or_else(|| v.get("id_token").and_then(|t| t.as_str()).and_then(jwt::email))
        .map(|e| e.to_lowercase())
}

fn refresh_token(v: &serde_json::Value) -> Option<&str> {
    v.pointer("/token/refresh_token").and_then(|t| t.as_str()).filter(|t| !t.is_empty())
}

fn looks_like_email(s: &str) -> bool {
    s.len() < 120
        && !s.contains(char::is_whitespace)
        && s.split_once('@').is_some_and(|(u, d)| !u.is_empty() && d.contains('.'))
}

fn b64(s: &str) -> Option<Vec<u8>> {
    let t = s.trim();
    base64::engine::general_purpose::STANDARD
        .decode(t)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(t.trim_end_matches('=')))
        .ok()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelQuota {
    pub name: String,
    pub remaining: f32,
    pub reset_secs: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserStatus {
    pub email: Option<String>,
    pub name: Option<String>,
    pub plan: Option<String>,
    pub models: Vec<ModelQuota>,
}

fn inner_status(raw: &[u8]) -> Option<Vec<u8>> {
    let outer = b64(std::str::from_utf8(raw).ok()?)?;
    let mut found = None;
    pb::walk(&outer, 4, &mut |fields| {
        if found.is_some() {
            return;
        }
        for (_, v) in fields {
            if let Some(t) = pb::text(v) {
                if t.len() > 40 {
                    if let Some(inner) = b64(t) {
                        if pb::parse(&inner).is_some() {
                            found = Some(inner);
                            return;
                        }
                    }
                }
            }
        }
    });
    found
}

pub fn parse_user_status(raw: &[u8]) -> Option<UserStatus> {
    let inner = inner_status(raw)?;
    let top = pb::parse(&inner)?;
    let mut st = UserStatus {
        name: pb::get(&top, 3).and_then(pb::text).map(str::to_string),
        email: pb::get(&top, 7).and_then(pb::text).filter(|s| looks_like_email(s)).map(str::to_lowercase),
        ..Default::default()
    };
    pb::walk(&inner, 6, &mut |fields| {
        if st.email.is_none() {
            if let Some(e) = fields.iter().filter_map(|(_, v)| pb::text(v)).find(|t| looks_like_email(t)) {
                st.email = Some(e.to_lowercase());
            }
        }
        if st.plan.is_none() {
            if let (Some(id), Some(name)) =
                (pb::get(fields, 1).and_then(pb::text), pb::get(fields, 2).and_then(pb::text))
            {
                if id.ends_with("-tier") && !name.is_empty() {
                    st.plan = Some(name.to_string());
                }
            }
        }
        let (Some(name), Some(Val::Bytes(q))) = (pb::get(fields, 1).and_then(pb::text), pb::get(fields, 15)) else {
            return;
        };
        let Some(qf) = pb::parse(q) else { return };
        let Some(Val::Fixed32(bits)) = pb::get(&qf, 1) else { return };
        let remaining = f32::from_bits(*bits);
        if !(0.0..=1.0).contains(&remaining) || st.models.iter().any(|m| m.name == name) {
            return;
        }
        let reset_secs = match pb::get(&qf, 2) {
            Some(Val::Bytes(t)) => pb::parse(t).and_then(|tf| match pb::get(&tf, 1) {
                Some(Val::Varint(s)) => i64::try_from(*s).ok(),
                _ => None,
            }),
            _ => None,
        };
        st.models.push(ModelQuota { name: name.to_string(), remaining, reset_secs });
    });
    (st.email.is_some() || !st.models.is_empty()).then_some(st)
}

fn classify(p: &ProcView) -> Option<ProcessKind> {
    let exe = p.exe.as_ref().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    let name = p.name.as_str();
    let main = |exe: &str| exe.contains("Contents/MacOS/") && !exe.contains("Frameworks/");
    if exe.contains("Antigravity IDE.app/") {
        return Some(if main(&exe) { ProcessKind::Ide } else { ProcessKind::Helper });
    }
    if exe.contains("Antigravity.app/") {
        return Some(if main(&exe) { ProcessKind::App } else { ProcessKind::Helper });
    }
    if name.eq_ignore_ascii_case("Antigravity.exe") || name == "antigravity" {
        return Some(ProcessKind::App);
    }
    if name.eq_ignore_ascii_case("Antigravity IDE.exe") || name == "antigravity-ide" {
        return Some(ProcessKind::Ide);
    }
    if name == "agy" || name.eq_ignore_ascii_case("agy.exe") {
        return Some(ProcessKind::Cli);
    }
    None
}

fn state_key(path: &Path, key: &str) -> Locator {
    Locator::StateKey { path: path.to_path_buf(), key: key.into() }
}

impl Antigravity {
    pub fn new(ctx: Ctx) -> Self {
        Antigravity { ctx }
    }

    fn credential(&self) -> Locator {
        Locator::Credential { service: SERVICE.into(), account: ACCOUNT.into() }
    }

    fn gemini_dir(&self) -> PathBuf {
        self.ctx.home.join(".gemini")
    }

    fn cli_token(&self) -> Locator {
        Locator::File { path: self.gemini_dir().join("antigravity-cli").join("antigravity-oauth-token") }
    }

    fn standalone_token(&self) -> Locator {
        Locator::File { path: self.gemini_dir().join("jetski-standalone-oauth-token") }
    }

    fn config_root(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.ctx.home.join("Library/Application Support")
        } else if cfg!(windows) {
            std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| self.ctx.home.join("AppData/Roaming"))
        } else {
            self.ctx.home.join(".config")
        }
    }

    pub(crate) fn state_dbs(&self) -> Vec<PathBuf> {
        let root = self.config_root();
        let mut out: Vec<PathBuf> = Vec::new();
        for d in APP_DIRS {
            let p = root.join(d).join("User/globalStorage/state.vscdb");
            if p.is_file() && !out.iter().any(|o| same_file(o, &p)) {
                out.push(p);
            }
        }
        out
    }

    fn status_of(&self, snap: &Snapshot, db: &Path) -> Option<UserStatus> {
        snap.data_for(&state_key(db, KEY_STATUS)).and_then(parse_user_status)
    }

    fn ide_identity(&self, snap: &Snapshot) -> Option<(PathBuf, UserStatus)> {
        self.state_dbs().into_iter().find_map(|db| {
            snap.data_for(&state_key(&db, KEY_OAUTH))?;
            let st = self.status_of(snap, &db)?;
            st.email.clone().map(|_| (db, st))
        })
    }

    fn apps(&self) -> Vec<(PathBuf, ProcessKind)> {
        let mut out = Vec::new();
        #[cfg(target_os = "macos")]
        for base in [PathBuf::from("/Applications"), self.ctx.home.join("Applications")] {
            for (name, kind) in [("Antigravity.app", ProcessKind::App), ("Antigravity IDE.app", ProcessKind::Ide)] {
                let p = base.join(name);
                if p.is_dir() {
                    out.push((p, kind));
                }
            }
        }
        #[cfg(target_os = "windows")]
        if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            for (dir, exe, kind) in [
                ("Antigravity", "Antigravity.exe", ProcessKind::App),
                ("Antigravity IDE", "Antigravity IDE.exe", ProcessKind::Ide),
            ] {
                let p = local.join("Programs").join(dir).join(exe);
                if p.is_file() {
                    out.push((p, kind));
                }
            }
        }
        #[cfg(target_os = "linux")]
        for p in ["/usr/share/antigravity/antigravity", "/opt/Antigravity/antigravity", "/usr/bin/antigravity"] {
            let p = PathBuf::from(p);
            if p.is_file() {
                out.push((p, ProcessKind::App));
            }
        }
        out
    }

    pub(crate) fn watch_sign_in(
        &self,
        cancel: &AtomicBool,
        out: OutputSink,
        timeout: Duration,
        poll: Duration,
        open_app: bool,
    ) -> Result<Snapshot> {
        let before = self.capture()?.map(|s| s.identity.key);
        if open_app {
            match self.apps().into_iter().min_by_key(|(_, k)| if *k == ProcessKind::Ide { 0 } else { 1 }) {
                Some((path, _)) => {
                    let name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Antigravity".into());
                    out(&format!("Opening {name}…"));
                    if let Err(e) = process::launch(&path) {
                        out(&format!("Could not open {name} ({e}). Open it yourself."));
                    }
                }
                None => out("Open Antigravity, or run agy in a terminal."),
            }
        }
        out(match before {
            Some(_) => "In Antigravity, open the account menu, choose Sign out, then sign in with the other Google account. AltFlip saves it as soon as the sign-in changes.",
            None => "Sign in to Antigravity with the Google account you want to add. AltFlip saves it as soon as the sign-in appears.",
        });
        self.wait_for_new_sign_in(before, cancel, out, timeout, poll)
    }

    fn wait_for_new_sign_in(
        &self,
        before: Option<String>,
        cancel: &AtomicBool,
        out: OutputSink,
        timeout: Duration,
        poll: Duration,
    ) -> Result<Snapshot> {
        let start = Instant::now();
        let mut said_signed_out = false;
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(Error::Cancelled("Sign-in was cancelled".into()));
            }
            if start.elapsed() > timeout {
                return Err(Error::Cancelled("Sign-in timed out".into()));
            }
            std::thread::sleep(poll);
            match self.capture() {
                Ok(Some(s)) if before.as_deref() != Some(s.identity.key.as_str()) => {
                    out(&format!("Signed in as {}.", s.identity.short()));
                    return Ok(s);
                }
                Ok(None) if !said_signed_out => {
                    said_signed_out = true;
                    out("Signed out. Now sign in with the other Google account.");
                }
                _ => {}
            }
        }
    }

    fn app_version(path: &Path) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            let out = std::process::Command::new("/usr/bin/plutil")
                .args(["-extract", "CFBundleShortVersionString", "raw", "-o", "-"])
                .arg(path.join("Contents/Info.plist"))
                .output()
                .ok()?;
            out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            None
        }
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

impl Provider for Antigravity {
    fn id(&self) -> ProviderId {
        ProviderId::Antigravity
    }

    fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    fn experimental(&self) -> bool {
        true
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            multiple_accounts: true,
            account_switching: true,
            live_switching: false,
            isolated_login: true,
            login_replaces_current: true,
            usage_monitoring: Support::Conditional,
            restart_required: Support::Conditional,
        }
    }

    fn detection(&self) -> Detection {
        let apps = self.apps();
        let agy = process::find_binary("agy", &[]);
        let installed = !apps.is_empty() || agy.is_some() || self.gemini_dir().join("antigravity").is_dir();
        let mut versions: Vec<String> = apps
            .iter()
            .filter_map(|(p, k)| {
                Self::app_version(p).map(|v| format!("{} {v}", if *k == ProcessKind::Ide { "IDE" } else { "App" }))
            })
            .collect();
        if let Some(v) = agy.as_deref().and_then(version_of) {
            versions.push(format!("agy {v}"));
        }
        Detection {
            installed,
            version: (!versions.is_empty()).then(|| versions.join(" · ")),
            binary_path: apps.first().map(|a| a.0.clone()).or(agy),
            config_path: Some(self.gemini_dir()),
            compatibility: if installed { Compatibility::Experimental } else { Compatibility::NotInstalled },
            note: installed.then(|| {
                "Experimental: switches the Antigravity sign-in used by the App, IDE and agy CLI. Running Antigravity apps must restart, and open agent conversations do not move to the new account.".into()
            }),
        }
    }

    fn locators(&self) -> Vec<(Locator, Role)> {
        let mut out = Vec::new();
        for db in self.state_dbs() {
            for key in [KEY_OAUTH, KEY_STATUS, KEY_AUTH_SYNC, KEY_CREDITS] {
                out.push((state_key(&db, key), Role::Secondary));
            }
        }
        out.push((self.credential(), Role::Secondary));
        out.push((self.cli_token(), Role::Secondary));
        out.push((self.standalone_token(), Role::Secondary));
        out
    }

    fn normalize(&self, snap: &mut Snapshot) {
        let ide = self.ide_identity(snap);
        let keychain = snap.data_for(&self.credential()).and_then(decode_payload);
        let email =
            ide.as_ref().and_then(|(_, s)| s.email.clone()).or_else(|| keychain.as_ref().and_then(payload_email));
        let keychain_rt = keychain.as_ref().and_then(refresh_token).map(str::to_string);
        let statuses: Vec<(PathBuf, Option<String>)> = self
            .state_dbs()
            .into_iter()
            .map(|db| {
                let e = self.status_of(snap, &db).and_then(|s| s.email);
                (db, e)
            })
            .collect();
        for r in snap.resources.iter_mut() {
            if r.data.is_none() {
                continue;
            }
            let keep = match &r.locator {
                Locator::StateKey { path, .. } => {
                    let db_email = statuses.iter().find(|(p, _)| p == path).and_then(|(_, e)| e.clone());
                    db_email.is_some() && db_email == email
                }
                loc if *loc == self.credential() => ide.is_none() || keychain.as_ref().and_then(payload_email) == email,
                _ => {
                    let side = r.data.as_deref().and_then(decode_payload);
                    match (&side, &email, &keychain_rt) {
                        (Some(s), Some(e), _) if payload_email(s).is_some() => {
                            payload_email(s).as_deref() == Some(e.as_str())
                        }
                        (Some(s), _, Some(t)) => refresh_token(s) == Some(t.as_str()),
                        _ => false,
                    }
                }
            };
            if !keep {
                r.data = None;
            }
        }
        match &ide {
            Some((db, _)) => {
                for r in snap.resources.iter_mut() {
                    if let Locator::StateKey { path, key } = &r.locator {
                        if path == db && (key == KEY_OAUTH || key == KEY_STATUS) && r.data.is_some() {
                            r.role = Role::Primary;
                        }
                    }
                }
            }
            None => {
                let cred = self.credential();
                for r in snap.resources.iter_mut() {
                    if r.locator == cred && r.data.is_some() {
                        r.role = Role::Primary;
                    }
                }
            }
        }
    }

    fn identify(&self, snap: &Snapshot) -> Result<Option<Identity>> {
        if let Some((_, st)) = self.ide_identity(snap) {
            let email = st.email.clone().unwrap_or_default();
            return Ok(Some(Identity {
                key: format!("google:{email}"),
                email: Some(email),
                display_name: st.name,
                organization: None,
                plan: st.plan,
                auth_method: Some("google".into()),
            }));
        }
        let Some(v) = snap.data_for(&self.credential()).and_then(decode_payload) else { return Ok(None) };
        let email = payload_email(&v).or_else(|| {
            let accounts = std::fs::read(self.gemini_dir().join("google_accounts.json")).ok();
            json(accounts.as_deref()).and_then(|a| a.get("active").and_then(|x| x.as_str()).map(str::to_lowercase))
        });
        let key = match (&email, refresh_token(&v)) {
            (Some(e), _) => format!("google:{e}"),
            (None, Some(t)) => format!("google-token:{}", short_hash(t.as_bytes())),
            (None, None) => return Ok(None),
        };
        Ok(Some(Identity {
            key,
            email,
            display_name: None,
            organization: None,
            plan: None,
            auth_method: v.get("auth_method").and_then(|m| m.as_str()).map(str::to_string),
        }))
    }

    fn health(&self, snap: &Snapshot) -> AuthHealth {
        if self.ide_identity(snap).is_some() {
            return AuthHealth::Ok;
        }
        let Some(v) = snap.data_for(&self.credential()).and_then(decode_payload) else {
            return AuthHealth::NeedsReauth;
        };
        if refresh_token(&v).is_some() {
            return AuthHealth::Ok;
        }
        match v
            .pointer("/token/expiry")
            .and_then(|e| e.as_str())
            .and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok())
        {
            Some(t) if t < chrono::Utc::now() => AuthHealth::Expired,
            Some(_) => AuthHealth::Ok,
            None => AuthHealth::Unknown,
        }
    }

    fn processes(&self) -> Vec<ProcessInfo> {
        process::find(classify)
    }

    fn mode(&self, procs: &[ProcessInfo]) -> SwitchMode {
        if procs.iter().any(|p| matches!(p.kind, ProcessKind::App | ProcessKind::Ide)) {
            SwitchMode::RestartRequired
        } else {
            SwitchMode::NewSessions
        }
    }

    fn login(&self, cancel: &AtomicBool, out: OutputSink) -> Result<Snapshot> {
        let ide = self.apps().into_iter().find(|(_, k)| *k == ProcessKind::Ide);
        let signed_in: Vec<PathBuf> = self
            .state_dbs()
            .into_iter()
            .filter(|db| matches!(altflip_core::statedb::read(db, KEY_OAUTH), Ok(Some(_))))
            .collect();
        let Some((ide_path, _)) = ide.filter(|_| !signed_in.is_empty()) else {
            return self.watch_sign_in(cancel, out, Duration::from_secs(900), Duration::from_secs(2), true);
        };
        let running: Vec<ProcessInfo> = self.processes().into_iter().filter(|p| p.kind == ProcessKind::Ide).collect();
        if !running.is_empty() {
            out("Closing Antigravity so it can sign in to another account…");
            for t in &running {
                let _ = process::request_quit(t);
            }
            let pids: Vec<u32> = running.iter().map(|p| p.pid).collect();
            if !process::wait_exit(&pids, Duration::from_secs(30)) {
                return Err(Error::Platform("Antigravity did not close. Quit it yourself, then try again.".into()));
            }
        }
        for db in &signed_in {
            for key in [KEY_OAUTH, KEY_STATUS, KEY_AUTH_SYNC, KEY_CREDITS] {
                altflip_core::statedb::delete(db, key)?;
            }
        }
        let before = self.capture()?.map(|s| s.identity.key);
        out("Signed Antigravity out. Opening it now…");
        if let Err(e) = process::launch(&ide_path) {
            out(&format!("Could not open Antigravity ({e}). Open it yourself."));
        }
        out("In Antigravity, choose Sign in with Google. Your browser opens; pick the other Google account there. AltFlip saves it as soon as the sign-in appears.");
        let res = self.wait_for_new_sign_in(before, cancel, out, Duration::from_secs(900), Duration::from_secs(2));
        if res.is_err() {
            out("Antigravity is still signed out. Switch to a saved account in AltFlip to sign it back in.");
        }
        res
    }

    fn restart_targets(&self, procs: &[ProcessInfo]) -> Vec<ProcessInfo> {
        procs.iter().filter(|p| matches!(p.kind, ProcessKind::App | ProcessKind::Ide)).cloned().collect()
    }

    fn usage(&self) -> Vec<UsageReading> {
        let mut out = Vec::new();
        for db in self.state_dbs() {
            let Ok(Some(raw)) = altflip_core::statedb::read(&db, KEY_STATUS) else { continue };
            let Some(st) = parse_user_status(&raw) else { continue };
            let Some(email) = st.email.clone() else { continue };
            let observed = std::fs::metadata(&db)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
                .unwrap_or_else(altflip_core::now_rfc3339);
            let now = chrono::Utc::now().timestamp();
            for m in &st.models {
                let reset_passed = m.reset_secs.is_some_and(|r| r < now);
                let used = if reset_passed { 0.0 } else { ((1.0 - f64::from(m.remaining)) * 100.0).clamp(0.0, 100.0) };
                out.push(UsageReading {
                    usage_type: format!("model:{}", m.name),
                    label: m.name.clone(),
                    used_percent: Some(used),
                    used_value: None,
                    limit_value: None,
                    window_minutes: None,
                    reset_at: m.reset_secs.filter(|_| !reset_passed).and_then(epoch_to_rfc3339),
                    source: LOCAL_SOURCE.into(),
                    confidence: "provider-reported".into(),
                    observed_at: observed.clone(),
                    account_key: Some(format!("google:{email}")),
                });
            }
            if !out.is_empty() {
                break;
            }
        }
        out
    }
}

pub const LIVE_SOURCE: &str = "Antigravity (live)";
pub const LOCAL_SOURCE: &str = "Antigravity (local)";

fn is_language_server(p: &ProcView) -> Option<&str> {
    let csrf = p.cmd.iter().find_map(|a| a.strip_prefix("--csrf_token="))?;
    let exe = p.exe.as_deref().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let name = p.name.to_lowercase();
    let ours = name == "agy" || exe.contains("antigravity") || exe.contains(".gemini");
    ours.then_some(csrf)
}

fn listening_ports(p: &ProcView) -> Vec<u16> {
    let mut ports: Vec<u16> = p
        .cmd
        .iter()
        .filter_map(|a| {
            let (k, v) = a.strip_prefix("--")?.split_once('=')?;
            if k.ends_with("port") {
                v.parse().ok()
            } else {
                None
            }
        })
        .collect();
    #[cfg(unix)]
    {
        let lsof = ["/usr/sbin/lsof", "/usr/bin/lsof"].into_iter().find(|p| Path::new(p).exists()).unwrap_or("lsof");
        if let Ok(out) = std::process::Command::new(lsof)
            .args(["-nP", "-a", "-iTCP", "-sTCP:LISTEN", "-p", &p.pid.to_string(), "-Fn"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
        {
            for l in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(port) = l.strip_prefix('n').and_then(|a| a.rsplit_once(':')).and_then(|(_, n)| n.parse().ok())
                {
                    ports.push(port);
                }
            }
        }
    }
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn language_server_call(port: u16, csrf: &str, method: &str) -> Option<serde_json::Value> {
    let tls = ureq::tls::TlsConfig::builder().disable_verification(true).build();
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(6)))
            .http_status_as_error(false)
            .tls_config(tls)
            .build(),
    );
    for scheme in ["http", "https"] {
        let url = format!("{scheme}://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/{method}");
        let Ok(mut res) = agent
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Connect-Protocol-Version", "1")
            .header("X-Codeium-Csrf-Token", csrf)
            .send("{}")
        else {
            continue;
        };
        if res.status() != 200 {
            continue;
        }
        if let Ok(v) = res.body_mut().read_json::<serde_json::Value>() {
            return Some(v);
        }
    }
    None
}

fn window_minutes(window: &str) -> Option<i64> {
    match window {
        "5h" => Some(300),
        "weekly" => Some(10_080),
        "daily" => Some(1_440),
        _ => None,
    }
}

fn window_label(window: &str, fallback: &str) -> String {
    match window {
        "5h" => "5-hour".into(),
        "weekly" => "Weekly".into(),
        "daily" => "Daily".into(),
        _ => fallback.trim_end_matches(" Remaining").to_string(),
    }
}

pub fn parse_quota_summary(v: &serde_json::Value, email: &str, now: chrono::DateTime<chrono::Utc>) -> Vec<UsageReading> {
    let observed = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let groups = v.pointer("/response/groups").and_then(|g| g.as_array());
    let mut out = Vec::new();
    for g in groups.into_iter().flatten() {
        let group = g
            .get("displayName")
            .and_then(|n| n.as_str())
            .unwrap_or("Models")
            .trim_end_matches(" Models")
            .trim_end_matches(" models")
            .to_string();
        for b in g.get("buckets").and_then(|b| b.as_array()).into_iter().flatten() {
            let Some(id) = b.get("bucketId").and_then(|i| i.as_str()) else { continue };
            let window = b.get("window").and_then(|w| w.as_str()).unwrap_or_default();
            let reset = b
                .get("resetTime")
                .and_then(|r| r.as_str())
                .and_then(|r| chrono::DateTime::parse_from_rfc3339(r).ok())
                .map(|r| r.with_timezone(&chrono::Utc));
            let frac = b.get("remainingFraction").and_then(|f| f.as_f64());
            if frac.is_none() && reset.is_none() {
                continue;
            }
            let reset_passed = reset.is_some_and(|r| r < now);
            let used = if reset_passed { 0.0 } else { ((1.0 - frac.unwrap_or(0.0)) * 100.0).clamp(0.0, 100.0) };
            let name = b.get("displayName").and_then(|n| n.as_str()).unwrap_or(id);
            out.push(UsageReading {
                usage_type: format!("quota:{id}"),
                label: format!("{group} · {}", window_label(window, name)),
                used_percent: Some(used),
                used_value: None,
                limit_value: None,
                window_minutes: window_minutes(window),
                reset_at: reset.filter(|_| !reset_passed).map(|r| r.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                source: LIVE_SOURCE.into(),
                confidence: "provider-reported".into(),
                observed_at: observed.clone(),
                account_key: Some(format!("google:{email}")),
            });
        }
    }
    out
}

pub fn parse_live_status(v: &serde_json::Value, now: chrono::DateTime<chrono::Utc>) -> Vec<UsageReading> {
    let Some(st) = v.get("userStatus") else { return Vec::new() };
    let Some(email) = st.get("email").and_then(|e| e.as_str()).filter(|e| !e.is_empty()) else { return Vec::new() };
    let observed = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let models = st.pointer("/cascadeModelConfigData/clientModelConfigs").and_then(|m| m.as_array());
    let mut out = Vec::new();
    for m in models.into_iter().flatten() {
        let Some(label) = m.get("label").and_then(|l| l.as_str()) else { continue };
        let Some(q) = m.get("quotaInfo") else { continue };
        let reset = q
            .get("resetTime")
            .and_then(|r| r.as_str())
            .and_then(|r| chrono::DateTime::parse_from_rfc3339(r).ok())
            .map(|r| r.with_timezone(&chrono::Utc));
        let frac = q.get("remainingFraction").and_then(|f| f.as_f64());
        if frac.is_none() && reset.is_none() {
            continue;
        }
        let reset_passed = reset.is_some_and(|r| r < now);
        let used = if reset_passed { 0.0 } else { ((1.0 - frac.unwrap_or(0.0)) * 100.0).clamp(0.0, 100.0) };
        out.push(UsageReading {
            usage_type: format!("model:{label}"),
            label: label.to_string(),
            used_percent: Some(used),
            used_value: None,
            limit_value: None,
            window_minutes: None,
            reset_at: reset.filter(|_| !reset_passed).map(|r| r.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            source: LIVE_SOURCE.into(),
            confidence: "provider-reported".into(),
            observed_at: observed.clone(),
            account_key: Some(format!("google:{email}")),
        });
    }
    out
}

pub fn live_usage() -> Vec<UsageReading> {
    let now = chrono::Utc::now();
    let mut out: Vec<UsageReading> = Vec::new();
    for p in process::snapshot() {
        let Some(csrf) = is_language_server(&p) else { continue };
        for port in listening_ports(&p) {
            let Some(status) = language_server_call(port, csrf, "GetUserStatus").filter(|v| v.get("userStatus").is_some())
            else {
                continue;
            };
            let Some(email) = status.pointer("/userStatus/email").and_then(|e| e.as_str()).filter(|e| !e.is_empty())
            else {
                break;
            };
            let key = Some(format!("google:{email}"));
            if out.iter().any(|r| r.account_key == key) {
                break;
            }
            let mut readings = language_server_call(port, csrf, "RetrieveUserQuotaSummary")
                .map(|v| parse_quota_summary(&v, email, now))
                .unwrap_or_default();
            if readings.is_empty() {
                readings = parse_live_status(&status, now);
            }
            out.extend(readings);
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pb::build;
    use crate::testutil::{ctx, jwt};

    #[test]
    fn parses_quota_summary_groups() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-16T11:00:00Z").unwrap().with_timezone(&chrono::Utc);
        let v = serde_json::json!({"response": {"groups": [
            {"displayName": "Claude and GPT models", "buckets": [
                {"bucketId": "3p-weekly", "displayName": "Weekly Limit Remaining", "window": "weekly", "remainingFraction": 0.92, "resetTime": "2026-11-20T15:40:03Z"},
                {"bucketId": "3p-5h", "displayName": "Five Hour Limit Remaining", "window": "5h", "resetTime": "2026-11-16T15:30:09Z"}
            ]},
            {"displayName": "Gemini Models", "buckets": [
                {"bucketId": "gemini-5h", "window": "5h", "remainingFraction": 0.2, "resetTime": "2026-11-16T10:00:00Z"}
            ]}
        ]}});
        let r = parse_quota_summary(&v, "a@b.c", now);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].usage_type, "quota:3p-weekly");
        assert_eq!(r[0].label, "Claude and GPT · Weekly");
        assert!((r[0].used_percent.unwrap() - 8.0).abs() < 0.001);
        assert_eq!(r[0].window_minutes, Some(10_080));
        assert_eq!(r[1].label, "Claude and GPT · 5-hour");
        assert_eq!(r[1].used_percent, Some(100.0));
        assert_eq!(r[2].label, "Gemini · 5-hour");
        assert_eq!(r[2].used_percent, Some(0.0));
        assert!(r[2].reset_at.is_none());
        assert_eq!(r[2].account_key.as_deref(), Some("google:a@b.c"));
    }

    #[test]
    fn parses_live_language_server_status() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-16T11:00:00Z").unwrap().with_timezone(&chrono::Utc);
        let v = serde_json::json!({"userStatus": {"email": "a@b.c", "cascadeModelConfigData": {"clientModelConfigs": [
            {"label": "Flash", "quotaInfo": {"remainingFraction": 0.75, "resetTime": "2026-11-16T15:00:00Z"}},
            {"label": "Spent", "quotaInfo": {"resetTime": "2026-11-16T15:00:00Z"}},
            {"label": "Old", "quotaInfo": {"remainingFraction": 0.1, "resetTime": "2026-11-16T10:00:00Z"}},
            {"label": "None", "quotaInfo": {}}
        ]}}});
        let r = parse_live_status(&v, now);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].used_percent, Some(25.0));
        assert_eq!(r[0].reset_at.as_deref(), Some("2026-11-16T15:00:00Z"));
        assert_eq!(r[0].account_key.as_deref(), Some("google:a@b.c"));
        assert_eq!(r[1].used_percent, Some(100.0));
        assert_eq!(r[2].used_percent, Some(0.0));
        assert!(r[2].reset_at.is_none());
        assert!(r.iter().all(|u| u.source == LIVE_SOURCE));
    }

    fn payload(email: &str, rt: &str) -> Vec<u8> {
        let json = serde_json::json!({"token": {"access_token": "a", "token_type": "Bearer", "refresh_token": rt, "expiry": "2030-01-01T00:00:00Z"}, "auth_method": "consumer", "id_token": jwt(serde_json::json!({"email": email}))});
        format!("{B64_PREFIX}{}", base64::engine::general_purpose::STANDARD.encode(json.to_string())).into_bytes()
    }

    fn status_blob(email: &str, models: &[(&str, f32, u64)]) -> Vec<u8> {
        let mut inner = Vec::new();
        build::uint(2, 1, &mut inner);
        build::bytes(3, b"Test User", &mut inner);
        build::bytes(7, email.as_bytes(), &mut inner);
        let mut list = Vec::new();
        for (name, frac, reset) in models {
            let mut ts = Vec::new();
            build::uint(1, *reset, &mut ts);
            let mut q = Vec::new();
            build::f32(1, *frac, &mut q);
            build::bytes(2, &ts, &mut q);
            let mut m = Vec::new();
            build::bytes(1, name.as_bytes(), &mut m);
            build::bytes(15, &q, &mut m);
            build::bytes(1, &m, &mut list);
        }
        let mut tier = Vec::new();
        build::bytes(1, b"g1-pro-tier", &mut tier);
        build::bytes(2, b"Google AI Pro", &mut tier);
        build::bytes(36, &list, &mut inner);
        build::bytes(37, &tier, &mut inner);
        let inner_b64 = base64::engine::general_purpose::STANDARD.encode(&inner);
        let mut val = Vec::new();
        build::bytes(1, inner_b64.as_bytes(), &mut val);
        let mut entry = Vec::new();
        build::bytes(1, b"userStatusSentinelKey", &mut entry);
        build::bytes(2, &val, &mut entry);
        let mut outer = Vec::new();
        build::bytes(1, &entry, &mut outer);
        base64::engine::general_purpose::STANDARD.encode(&outer).into_bytes()
    }

    fn ide_db(ctx: &Ctx) -> PathBuf {
        let a = Antigravity::new(ctx.clone());
        let dir = a.config_root().join("Antigravity IDE/User/globalStorage");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("state.vscdb");
        let c = rusqlite::Connection::open(&p).unwrap();
        c.execute_batch("CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)").unwrap();
        p
    }

    fn sign_in_ide(db: &Path, email: &str, token: &str) {
        altflip_core::statedb::write(db, KEY_OAUTH, format!("oauth-{token}").as_bytes()).unwrap();
        altflip_core::statedb::write(db, KEY_STATUS, &status_blob(email, &[("Gemini Flash", 0.75, 1_900_000_000)]))
            .unwrap();
    }

    #[test]
    fn parses_user_status() {
        let st =
            parse_user_status(&status_blob("Me@X.dev", &[("Gemini Flash", 0.25, 1_900_000_000), ("Claude", 1.0, 5)]))
                .unwrap();
        assert_eq!(st.email.as_deref(), Some("me@x.dev"));
        assert_eq!(st.name.as_deref(), Some("Test User"));
        assert_eq!(st.plan.as_deref(), Some("Google AI Pro"));
        assert_eq!(st.models.len(), 2);
        assert_eq!(
            st.models[0],
            ModelQuota { name: "Gemini Flash".into(), remaining: 0.25, reset_secs: Some(1_900_000_000) }
        );
        assert!(parse_user_status(b"not base64!").is_none());
    }

    #[test]
    fn ide_sign_in_identifies_switches_and_reports_quota() {
        let (_d, ctx) = ctx();
        let db = ide_db(&ctx);
        let a = Antigravity::new(ctx.clone());
        ctx.io.write(&a.credential(), &payload("old-cli@x.dev", "r0")).unwrap();
        sign_in_ide(&db, "main@x.dev", "t1");
        let main = a.capture().unwrap().unwrap();
        assert_eq!(main.identity.key, "google:main@x.dev");
        assert_eq!(main.identity.plan.as_deref(), Some("Google AI Pro"));
        assert!(main.data_for(&a.credential()).is_none());
        assert!(main.primary().all(|r| matches!(r.locator, Locator::StateKey { .. })));
        sign_in_ide(&db, "alt@x.dev", "t2");
        let alt = a.capture().unwrap().unwrap();
        assert_eq!(alt.identity.key, "google:alt@x.dev");
        a.apply(&main).unwrap();
        assert!(a.verify(&main).is_ok());
        assert!(a.verify(&alt).is_err());
        let u = a.usage();
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].account_key.as_deref(), Some("google:main@x.dev"));
        assert_eq!(u[0].used_percent, Some(25.0));
    }

    #[test]
    fn guided_sign_in_detects_ide_account_change() {
        let (_d, ctx) = ctx();
        let db = ide_db(&ctx);
        sign_in_ide(&db, "main@x.dev", "t1");
        let a = Antigravity::new(ctx.clone());
        let db2 = db.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(120));
            altflip_core::statedb::delete(&db2, KEY_OAUTH).unwrap();
            std::thread::sleep(Duration::from_millis(120));
            sign_in_ide(&db2, "alt@x.dev", "t2");
        });
        let cancel = AtomicBool::new(false);
        let s = a.watch_sign_in(&cancel, &|_| {}, Duration::from_secs(5), Duration::from_millis(30), false).unwrap();
        t.join().unwrap();
        assert_eq!(s.identity.key, "google:alt@x.dev");
        cancel.store(true, Ordering::SeqCst);
        assert!(a.watch_sign_in(&cancel, &|_| {}, Duration::from_secs(5), Duration::from_millis(10), false).is_err());
    }

    #[test]
    fn keychain_only_setup_still_works() {
        let (_d, ctx) = ctx();
        let a = Antigravity::new(ctx.clone());
        assert!(a.capture().unwrap().is_none());
        ctx.io.write(&a.credential(), &payload("Main@x.dev", "r1")).unwrap();
        ctx.io.write(&a.cli_token(), &payload("main@x.dev", "r1")).unwrap();
        ctx.io
            .write(&a.standalone_token(), br#"{"token":{"refresh_token":"zz"},"id_token_email":"other@x.dev"}"#)
            .unwrap();
        let main = a.capture().unwrap().unwrap();
        assert_eq!(main.identity.key, "google:main@x.dev");
        assert!(main.data_for(&a.cli_token()).is_some());
        assert!(main.data_for(&a.standalone_token()).is_none());
        ctx.io.write(&a.credential(), &payload("alt@x.dev", "r2")).unwrap();
        a.apply(&main).unwrap();
        assert!(a.verify(&main).is_ok());
    }

    #[test]
    fn restart_needed_only_for_gui() {
        let (_d, ctx) = ctx();
        let a = Antigravity::new(ctx);
        let p = |kind| ProcessInfo { pid: 1, name: "x".into(), path: None, kind };
        assert_eq!(a.mode(&[p(ProcessKind::Cli)]), SwitchMode::NewSessions);
        assert_eq!(a.mode(&[p(ProcessKind::App), p(ProcessKind::Helper)]), SwitchMode::RestartRequired);
        assert_eq!(a.restart_targets(&[p(ProcessKind::App), p(ProcessKind::Helper)]).len(), 1);
    }
}
