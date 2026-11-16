use crate::{parse_semver, version_of, Ctx, Detection, Provider};
use altflip_core::process::{self, ProcView};
use altflip_core::resource::{Locator, Resource, Role, Snapshot};
use altflip_core::{
    AuthHealth, Capabilities, Compatibility, Error, Identity, ProcessInfo, ProcessKind, ProviderId, Result, Support,
    SwitchMode,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const APP: &str = "opencode";

pub struct OpenCode {
    ctx: Ctx,
    data_dir: PathBuf,
    state_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Credential {
    pub id: String,
    pub integration: String,
    pub label: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
struct Service {
    url: String,
    password: Option<String>,
}

fn classify(p: &ProcView) -> Option<ProcessKind> {
    let exe = p.exe.as_ref().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    if exe.contains("OpenCode.app") {
        return Some(if p.name == "OpenCode" { ProcessKind::App } else { ProcessKind::Helper });
    }
    let name = p.name.to_lowercase();
    if name == "opencode" || name == "opencode.exe" {
        if p.cmd.iter().any(|a| a == "serve") {
            return Some(ProcessKind::Helper);
        }
        return Some(ProcessKind::Cli);
    }
    None
}

fn integration_name(id: &str) -> String {
    match id {
        "opencode" => "OpenCode Console".into(),
        "opencode-go" => "OpenCode Go".into(),
        other => other.into(),
    }
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub(crate) fn read_credentials(db: &Path) -> Result<Vec<Credential>> {
    let conn = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| Error::Platform(format!("Cannot open OpenCode's database: {e}")))?;
    conn.busy_timeout(Duration::from_secs(3)).map_err(|e| Error::Platform(e.to_string()))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, integration_id, label, COALESCE(active, 0) FROM credential \
             WHERE integration_id IS NOT NULL ORDER BY integration_id, time_created, id",
        )
        .map_err(|e| Error::Platform(format!("OpenCode's database has an unexpected layout: {e}")))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Credential { id: r.get(0)?, integration: r.get(1)?, label: r.get(2)?, active: r.get::<_, i64>(3)? != 0 })
        })
        .map_err(|e| Error::Platform(e.to_string()))?;
    rows.collect::<std::result::Result<Vec<_>, _>>().map_err(|e| Error::Platform(e.to_string()))
}

fn group(creds: &[Credential]) -> BTreeMap<String, Vec<&Credential>> {
    let mut m: BTreeMap<String, Vec<&Credential>> = BTreeMap::new();
    for c in creds {
        m.entry(c.integration.clone()).or_default().push(c);
    }
    m
}

fn selected<'a>(list: &[&'a Credential]) -> Option<&'a Credential> {
    let active: Vec<_> = list.iter().filter(|c| c.active).collect();
    match (active.as_slice(), list) {
        ([one], _) => Some(**one),
        ([], [only]) => Some(*only),
        _ => None,
    }
}

fn switchable(creds: &[Credential]) -> Vec<String> {
    let groups = group(creds);
    let multi: Vec<String> = groups.iter().filter(|(_, v)| v.len() > 1).map(|(k, _)| k.clone()).collect();
    if multi.is_empty() {
        groups.into_keys().collect()
    } else {
        multi
    }
}

pub(crate) fn snapshot_for(creds: &[Credential], choice: &BTreeMap<String, String>) -> Option<Snapshot> {
    let keys = switchable(creds);
    if keys.is_empty() {
        return None;
    }
    let mut resources = Vec::new();
    let mut parts = Vec::new();
    let mut labels = Vec::new();
    for k in &keys {
        let id = choice.get(k)?;
        let cred = creds.iter().find(|c| &c.id == id && &c.integration == k)?;
        parts.push(format!("{k}={id}"));
        labels.push(cred.label.clone());
        resources.push(Resource {
            locator: Locator::Selection { app: APP.into(), key: k.clone() },
            role: Role::Primary,
            data: Some(id.as_bytes().to_vec()),
        });
    }
    let identity = Identity {
        key: format!("opencode:{}", parts.join(",")),
        email: None,
        display_name: Some(labels.join(" · ")),
        organization: Some(keys.iter().map(|k| integration_name(k)).collect::<Vec<_>>().join(", ")),
        plan: None,
        auth_method: Some("opencode".into()),
    };
    Some(Snapshot::new(ProviderId::OpenCode, identity, resources))
}

pub(crate) fn current_choice(creds: &[Credential]) -> BTreeMap<String, String> {
    group(creds).iter().filter_map(|(k, v)| selected(v).map(|c| (k.clone(), c.id.clone()))).collect()
}

pub(crate) fn importable(creds: &[Credential]) -> Vec<Snapshot> {
    let base = current_choice(creds);
    let groups = group(creds);
    let mut out: Vec<Snapshot> = Vec::new();
    for k in switchable(creds) {
        for c in groups.get(&k).into_iter().flatten() {
            let mut choice = base.clone();
            choice.insert(k.clone(), c.id.clone());
            if let Some(s) = snapshot_for(creds, &choice) {
                if !out.iter().any(|o| o.identity.key == s.identity.key) {
                    out.push(s);
                }
            }
        }
    }
    out
}

fn read_service(state_dir: &Path) -> Option<Service> {
    let dir = state_dir.join("opencode");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .is_some_and(|n| n == "service.json" || (n.starts_with("service-") && n.ends_with(".json")))
        })
        .collect();
    files.sort_by_key(|p| p.file_name() != Some(std::ffi::OsStr::new("service.json")));
    files.into_iter().find_map(|p| {
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(p).ok()?).ok()?;
        let url = v.get("url")?.as_str()?.trim_end_matches('/').to_string();
        let local = ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"].iter().any(|h| url.starts_with(h));
        if !local {
            return None;
        }
        Some(Service { url, password: v.get("password").and_then(|p| p.as_str()).map(str::to_string) })
    })
}

fn activate(state_dir: &Path, credential_id: &str) -> Result<()> {
    if !safe_id(credential_id) {
        return Err(Error::Invalid("The saved OpenCode credential id is not valid".into()));
    }
    let svc = read_service(state_dir).ok_or_else(|| {
        Error::Platform(
            "OpenCode's background service is not running. Open OpenCode or run any opencode command, then try again."
                .into(),
        )
    })?;
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).http_status_as_error(false).build(),
    );
    let mut req =
        agent.post(&format!("{}/api/credential/{credential_id}/activate", svc.url)).header("User-Agent", "AltFlip");
    if let Some(pw) = &svc.password {
        use base64::Engine;
        let token = base64::engine::general_purpose::STANDARD.encode(format!("opencode:{pw}"));
        req = req.header("Authorization", &format!("Basic {token}"));
    }
    let resp = req.send_empty().map_err(|e| {
        Error::Platform(format!(
            "Cannot reach OpenCode's background service ({e}). Open OpenCode or run any opencode command, then try again."
        ))
    })?;
    match resp.status().as_u16() {
        200..=299 => Ok(()),
        401 | 403 => Err(Error::Platform("OpenCode's background service refused AltFlip's request".into())),
        404 => Err(Error::Credential("OpenCode no longer has this sign-in. It may have been logged out.".into())),
        s => Err(Error::Platform(format!("OpenCode's background service answered {s}"))),
    }
}

impl OpenCode {
    pub fn new(ctx: Ctx) -> Self {
        let env_dir = |var: &str, fallback: &[&str]| {
            std::env::var_os(var)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| fallback.iter().fold(ctx.home.clone(), |p, s| p.join(s)))
        };
        let data_dir = env_dir("XDG_DATA_HOME", &[".local", "share"]);
        let state_dir = env_dir("XDG_STATE_HOME", &[".local", "state"]);
        OpenCode { ctx, data_dir, state_dir }
    }

    #[cfg(test)]
    pub(crate) fn with_dirs(ctx: Ctx, data_dir: PathBuf, state_dir: PathBuf) -> Self {
        OpenCode { ctx, data_dir, state_dir }
    }

    fn db(&self) -> PathBuf {
        self.data_dir.join("opencode").join("opencode.db")
    }

    fn binary(&self) -> Option<PathBuf> {
        process::find_binary("opencode", &[self.ctx.home.join(".opencode/bin")])
    }

    fn credentials(&self) -> Result<Vec<Credential>> {
        if !self.db().is_file() {
            return Ok(Vec::new());
        }
        read_credentials(&self.db())
    }

    fn select(&self, snap: &Snapshot, strict: bool) -> Result<()> {
        if snap.provider != ProviderId::OpenCode {
            return Err(Error::Invalid("Saved account belongs to a different provider".into()));
        }
        let creds = self.credentials()?;
        let now = current_choice(&creds);
        for r in &snap.resources {
            let Locator::Selection { key, .. } = &r.locator else { continue };
            let Some(target) = r.data.as_deref().map(|d| String::from_utf8_lossy(d).to_string()) else {
                if strict {
                    return Err(Error::Invalid(format!(
                        "The saved account has no choice for {}",
                        integration_name(key)
                    )));
                }
                continue;
            };
            if !creds.iter().any(|c| c.id == target && &c.integration == key) {
                return Err(Error::Credential(format!(
                    "OpenCode no longer has this {} sign-in. Add it again with opencode auth login.",
                    integration_name(key)
                )));
            }
            if now.get(key) != Some(&target) {
                activate(&self.state_dir, &target)?;
            }
        }
        Ok(())
    }
}

impl Provider for OpenCode {
    fn id(&self) -> ProviderId {
        ProviderId::OpenCode
    }

    fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            multiple_accounts: true,
            account_switching: true,
            live_switching: true,
            isolated_login: false,
            login_replaces_current: false,
            usage_monitoring: Support::No,
            restart_required: Support::No,
        }
    }

    fn detection(&self) -> Detection {
        let bin = self.binary();
        let version = bin.as_deref().and_then(version_of);
        let has_db = self.db().is_file();
        let installed = bin.is_some() || has_db;
        let major = version.as_deref().and_then(parse_semver).map(|v| v.0);
        let (compatibility, note) = if !installed {
            (Compatibility::NotInstalled, None)
        } else if major.is_some_and(|m| m < 2) {
            (
                Compatibility::Unsupported,
                Some("OpenCode 1.x keeps sign-ins in auth.json. Update to OpenCode 2 to switch with AltFlip.".into()),
            )
        } else if !has_db {
            (Compatibility::Unsupported, Some("OpenCode has not created its database yet. Run opencode once.".into()))
        } else {
            (Compatibility::Supported, None)
        };
        Detection {
            installed,
            version,
            binary_path: bin,
            config_path: Some(self.data_dir.join("opencode")),
            compatibility,
            note,
        }
    }

    fn mode(&self, _procs: &[ProcessInfo]) -> SwitchMode {
        SwitchMode::Immediate
    }

    fn locators(&self) -> Vec<(Locator, Role)> {
        Vec::new()
    }

    fn identify(&self, snap: &Snapshot) -> Result<Option<Identity>> {
        Ok(if snap.identity.key.is_empty() { None } else { Some(snap.identity.clone()) })
    }

    fn health(&self, snap: &Snapshot) -> AuthHealth {
        if snap.primary().next().is_some() && snap.primary().all(|r| r.data.is_some()) {
            AuthHealth::Ok
        } else {
            AuthHealth::NeedsReauth
        }
    }

    fn processes(&self) -> Vec<ProcessInfo> {
        process::find(classify)
    }

    fn capture(&self) -> Result<Option<Snapshot>> {
        let creds = self.credentials()?;
        Ok(snapshot_for(&creds, &current_choice(&creds)))
    }

    fn raw_capture(&self) -> Result<Snapshot> {
        let creds = self.credentials()?;
        let choice = current_choice(&creds);
        let resources = switchable(&creds)
            .into_iter()
            .map(|k| Resource {
                data: choice.get(&k).map(|id| id.as_bytes().to_vec()),
                locator: Locator::Selection { app: APP.into(), key: k },
                role: Role::Primary,
            })
            .collect();
        Ok(Snapshot::new(ProviderId::OpenCode, Identity::default(), resources))
    }

    fn apply(&self, snap: &Snapshot) -> Result<()> {
        self.select(snap, true)
    }

    fn restore(&self, snap: &Snapshot) -> Result<()> {
        self.select(snap, false)
    }

    fn verify(&self, expected: &Snapshot) -> Result<String> {
        let live = self
            .capture()?
            .ok_or_else(|| Error::Verification("OpenCode reports no stored sign-in after switching".into()))?;
        if live.identity.key != expected.identity.key || !expected.same_primary_data(&live) {
            return Err(Error::Verification(format!(
                "OpenCode is using {} instead of {}",
                live.identity.short(),
                expected.identity.short()
            )));
        }
        Ok(format!("OpenCode's database shows {} as the active sign-in", expected.identity.short()))
    }

    fn import(&self) -> Result<Vec<Snapshot>> {
        let creds = self.credentials()?;
        if creds.is_empty() {
            return Err(Error::NotFound(
                "OpenCode has no stored sign-ins. Run opencode auth login in Terminal first.".into(),
            ));
        }
        Ok(importable(&creds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::ctx;
    use std::io::{BufRead, BufReader, Write};

    fn make_db(path: &Path, rows: &[(&str, &str, &str, Option<i64>, i64)]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let c = rusqlite::Connection::open(path).unwrap();
        c.execute_batch(
            "CREATE TABLE credential (id text PRIMARY KEY, integration_id text, label text NOT NULL, value text NOT NULL, \
             connector_id text, method_id text, active integer, time_created integer NOT NULL, time_updated integer NOT NULL);",
        )
        .unwrap();
        for (id, integ, label, active, t) in rows {
            c.execute(
                "INSERT INTO credential VALUES (?1, ?2, ?3, 'secret', NULL, NULL, ?4, ?5, ?5)",
                rusqlite::params![id, integ, label, active, t],
            )
            .unwrap();
        }
    }

    fn sample(db: &Path) {
        make_db(
            db,
            &[
                ("cred_api", "opencode", "API key", Some(0), 1),
                ("cred_thk", "opencode", "THK", Some(1), 2),
                ("cred_go", "opencode-go", "OpenCode Go", Some(1), 3),
                ("cred_cf", "cloudflare-workers-ai", "API key", None, 4),
            ],
        );
    }

    fn serve_once(state: &Path, db: PathBuf, password: &str) -> std::thread::JoinHandle<String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::fs::create_dir_all(state.join("opencode")).unwrap();
        std::fs::write(
            state.join("opencode/service.json"),
            serde_json::json!({"url": format!("http://127.0.0.1:{port}"), "password": password, "pid": 1}).to_string(),
        )
        .unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(s.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            let path = head.split_whitespace().nth(1).unwrap_or_default().to_string();
            let id = path.trim_start_matches("/api/credential/").trim_end_matches("/activate").to_string();
            let c = rusqlite::Connection::open(&db).unwrap();
            let integ: String =
                c.query_row("SELECT integration_id FROM credential WHERE id = ?1", [&id], |r| r.get(0)).unwrap();
            c.execute("UPDATE credential SET active = 0 WHERE integration_id = ?1", [&integ]).unwrap();
            c.execute("UPDATE credential SET active = 1 WHERE id = ?1", [&id]).unwrap();
            s.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            head
        })
    }

    #[test]
    fn identity_tracks_only_switchable_integrations() {
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("opencode.db");
        sample(&db);
        let creds = read_credentials(&db).unwrap();
        let s = snapshot_for(&creds, &current_choice(&creds)).unwrap();
        assert_eq!(s.identity.key, "opencode:opencode=cred_thk");
        assert_eq!(s.identity.display_name.as_deref(), Some("THK"));
        assert_eq!(s.resources.len(), 1);
        let all = importable(&creds);
        let keys: Vec<_> = all.iter().map(|s| s.identity.key.as_str()).collect();
        assert_eq!(keys, vec!["opencode:opencode=cred_api", "opencode:opencode=cred_thk"]);
    }

    #[test]
    fn single_credentials_still_identify_and_ambiguous_active_is_unknown() {
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("a.db");
        make_db(&db, &[("c1", "x", "Only", None, 1)]);
        let creds = read_credentials(&db).unwrap();
        assert_eq!(snapshot_for(&creds, &current_choice(&creds)).unwrap().identity.key, "opencode:x=c1");
        let db2 = d.path().join("b.db");
        make_db(&db2, &[("c1", "x", "A", None, 1), ("c2", "x", "B", None, 2)]);
        let creds = read_credentials(&db2).unwrap();
        assert!(snapshot_for(&creds, &current_choice(&creds)).is_none());
    }

    #[test]
    fn switch_activates_through_service_and_verifies() {
        let (_d, c) = ctx();
        let data = c.home.join("data");
        let state = c.home.join("state");
        let db = data.join("opencode/opencode.db");
        sample(&db);
        let p = OpenCode::with_dirs(c, data, state.clone());
        let target =
            p.import().unwrap().into_iter().find(|s| s.identity.display_name.as_deref() == Some("API key")).unwrap();
        let server = serve_once(&state, db.clone(), "pw");
        p.apply(&target).unwrap();
        let head = server.join().unwrap();
        assert!(head.starts_with("POST /api/credential/cred_api/activate "));
        use base64::Engine;
        let auth = base64::engine::general_purpose::STANDARD.encode("opencode:pw");
        assert!(head.contains(&format!("Basic {auth}")));
        assert!(p.verify(&target).unwrap().contains("API key"));
        let creds = read_credentials(&db).unwrap();
        assert_eq!(current_choice(&creds).get("opencode-go").map(String::as_str), Some("cred_go"));
    }

    #[test]
    fn already_selected_makes_no_request_and_missing_service_is_reported() {
        let (_d, c) = ctx();
        let data = c.home.join("data");
        let db = data.join("opencode/opencode.db");
        sample(&db);
        let p = OpenCode::with_dirs(c.clone(), data, c.home.join("nostate"));
        let live = p.capture().unwrap().unwrap();
        p.apply(&live).unwrap();
        let other = p.import().unwrap().into_iter().find(|s| s.identity.key != live.identity.key).unwrap();
        let err = p.apply(&other).unwrap_err().to_string();
        assert!(err.contains("background service is not running"), "{err}");
        assert!(p.raw_capture().unwrap().same_primary_data(&live));
    }

    #[test]
    fn selection_locators_are_never_written_as_files() {
        let (_d, c) = ctx();
        let loc = Locator::Selection { app: APP.into(), key: "opencode".into() };
        assert!(c.io.write(&loc, b"x").is_err());
        assert!(c.io.read(&loc).is_err());
        assert!(!safe_id("../x"));
        assert!(safe_id("cred_1aa6d0134001OJd00eEJPch5O4"));
    }

    #[test]
    fn classifies_service_as_helper() {
        let v = |name: &str, cmd: &[&str]| ProcView {
            pid: 1,
            name: name.into(),
            exe: None,
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
        };
        assert_eq!(classify(&v("opencode", &["opencode", "serve", "--service"])), Some(ProcessKind::Helper));
        assert_eq!(classify(&v("opencode", &["opencode"])), Some(ProcessKind::Cli));
        assert_eq!(classify(&v("node", &["node"])), None);
    }
}
