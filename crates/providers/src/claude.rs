use crate::{json, parse_semver, run_login, scratch_dir, version_of, Ctx, Detection, OutputSink, Provider};
use altflip_core::process::{self, ProcView};
use altflip_core::resource::{Locator, Resource, Role, Snapshot};
use altflip_core::{
    short_hash, AuthHealth, Capabilities, Compatibility, Error, Identity, ProcessInfo, ProcessKind, ProviderId, Result,
    Support, UsageReading,
};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
const MIN_TESTED: (u64, u64, u64) = (1, 0, 0);

pub struct Claude {
    ctx: Ctx,
    use_keychain: bool,
}

fn username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::process::Command::new("id")
                .arg("-un")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "user".into())
}

impl Claude {
    pub fn new(ctx: Ctx) -> Self {
        Claude { ctx, use_keychain: cfg!(target_os = "macos") }
    }

    #[cfg(test)]
    pub fn with_keychain(ctx: Ctx, use_keychain: bool) -> Self {
        Claude { ctx, use_keychain }
    }

    fn config_dir(&self) -> PathBuf {
        self.ctx.home.join(".claude")
    }

    fn global_config(&self) -> PathBuf {
        self.ctx.home.join(".claude.json")
    }

    fn keychain(&self) -> Locator {
        Locator::Credential { service: KEYCHAIN_SERVICE.into(), account: username() }
    }

    fn cred_file(&self) -> Locator {
        Locator::File { path: self.config_dir().join(".credentials.json") }
    }

    fn account_key(&self) -> Locator {
        Locator::JsonKey { path: self.global_config(), key: "oauthAccount".into() }
    }

    fn binary(&self) -> Option<PathBuf> {
        process::find_binary("claude", &[self.config_dir().join("local")])
    }

    fn creds<'a>(&self, snap: &'a Snapshot) -> Option<&'a [u8]> {
        snap.primary().find(|r| !matches!(r.locator, Locator::JsonKey { .. })).and_then(|r| r.data.as_deref())
    }
}

fn classify(p: &ProcView) -> Option<ProcessKind> {
    let exe = p.exe.as_ref().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    if exe.contains("Claude.app") || exe.contains("AnthropicClaude") {
        return None;
    }
    let name = p.name.to_lowercase();
    if name == "claude" || name == "claude.exe" {
        return Some(ProcessKind::Cli);
    }
    if (name == "node" || name == "node.exe" || name == "bun")
        && p.cmd.iter().any(|a| a.contains("@anthropic-ai/claude-code") || a.ends_with("claude-code/cli.js"))
    {
        return Some(ProcessKind::Cli);
    }
    None
}

impl Provider for Claude {
    fn id(&self) -> ProviderId {
        ProviderId::Claude
    }

    fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            multiple_accounts: true,
            account_switching: true,
            live_switching: false,
            isolated_login: true,
            login_replaces_current: false,
            usage_monitoring: Support::Conditional,
            restart_required: Support::Conditional,
        }
    }

    fn detection(&self) -> Detection {
        let bin = self.binary();
        let version = bin.as_deref().and_then(version_of);
        let has_config = self.global_config().is_file() || self.config_dir().is_dir();
        let installed = bin.is_some() || has_config;
        let (compatibility, note) = if !installed {
            (Compatibility::NotInstalled, None)
        } else if std::env::var_os("CLAUDE_CONFIG_DIR").is_some() {
            (
                Compatibility::Untested,
                Some(
                    "CLAUDE_CONFIG_DIR is set for AltFlip. AltFlip manages the default ~/.claude profile only.".into(),
                ),
            )
        } else {
            match version.as_deref().and_then(parse_semver) {
                Some(v) if v >= MIN_TESTED => (Compatibility::Supported, None),
                Some(_) => (Compatibility::Untested, Some("This Claude Code version has not been validated.".into())),
                None => (
                    Compatibility::Supported,
                    Some("The claude command was not found; AltFlip uses the existing configuration.".into()),
                ),
            }
        };
        Detection { installed, version, binary_path: bin, config_path: Some(self.config_dir()), compatibility, note }
    }

    fn locators(&self) -> Vec<(Locator, Role)> {
        if self.use_keychain {
            vec![
                (self.keychain(), Role::Primary),
                (self.account_key(), Role::Primary),
                (self.cred_file(), Role::Secondary),
            ]
        } else {
            vec![(self.cred_file(), Role::Primary), (self.account_key(), Role::Primary)]
        }
    }

    fn normalize(&self, snap: &mut Snapshot) {
        if !self.use_keychain {
            return;
        }
        let kc = self.keychain();
        let file = self.cred_file();
        if snap.data_for(&kc).is_none() && snap.data_for(&file).is_some() {
            for r in snap.resources.iter_mut() {
                if r.locator == kc {
                    r.role = Role::Secondary;
                } else if r.locator == file {
                    r.role = Role::Primary;
                }
            }
        }
    }

    fn identify(&self, snap: &Snapshot) -> Result<Option<Identity>> {
        let creds = json(self.creds(snap));
        let account = json(snap.data_for(&self.account_key()));
        if creds.is_none() && account.is_none() {
            return Ok(None);
        }
        let oauth = creds.as_ref().and_then(|c| c.get("claudeAiOauth"));
        let s = |v: Option<&serde_json::Value>, k: &str| {
            v.and_then(|v| v.get(k)).and_then(|x| x.as_str()).map(str::to_string)
        };
        let uuid = s(account.as_ref(), "accountUuid");
        let org_uuid = s(account.as_ref(), "organizationUuid");
        let email = s(account.as_ref(), "emailAddress");
        let key = match (&uuid, &email) {
            (Some(u), _) => format!("claude:{u}:{}", org_uuid.clone().unwrap_or_default()),
            (None, Some(e)) => format!("claude-email:{}", e.to_lowercase()),
            (None, None) => match s(oauth, "refreshToken") {
                Some(t) => format!("claude-token:{}", short_hash(t.as_bytes())),
                None => return Ok(None),
            },
        };
        let plan = s(oauth, "subscriptionType")
            .or_else(|| s(account.as_ref(), "billingType"))
            .or_else(|| s(account.as_ref(), "seatTier"));
        Ok(Some(Identity {
            key,
            email,
            display_name: s(account.as_ref(), "displayName").or_else(|| s(account.as_ref(), "fullName")),
            organization: s(account.as_ref(), "organizationName"),
            plan,
            auth_method: Some(if oauth.is_some() { "claude.ai".into() } else { "console".into() }),
        }))
    }

    fn health(&self, snap: &Snapshot) -> AuthHealth {
        let Some(creds) = json(self.creds(snap)) else { return AuthHealth::NeedsReauth };
        let Some(oauth) = creds.get("claudeAiOauth") else { return AuthHealth::Unknown };
        let has_refresh = oauth.get("refreshToken").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());
        let expires = oauth.get("expiresAt").and_then(|v| v.as_i64());
        let now = chrono::Utc::now().timestamp_millis();
        match (has_refresh, expires) {
            (true, _) => AuthHealth::Ok,
            (false, Some(e)) if e < now => AuthHealth::Expired,
            (false, Some(e)) if e - now < 86_400_000 => AuthHealth::Expiring,
            (false, Some(_)) => AuthHealth::Ok,
            (false, None) => AuthHealth::Unknown,
        }
    }

    fn processes(&self) -> Vec<ProcessInfo> {
        process::find(classify)
    }

    fn usage(&self) -> Vec<UsageReading> {
        use crate::claude_usage;
        let now = chrono::Utc::now().timestamp() as f64;
        let desktop = claude_usage::desktop_dir(&self.ctx.home);
        let sessions = claude_usage::desktop_sessions(&desktop);
        let mut out = claude_usage::status_line(&self.config_dir(), &sessions, now);
        out.extend(claude_usage::plan_history(&desktop, &sessions, now));
        out
    }

    fn verify(&self, expected: &Snapshot) -> Result<String> {
        let live = self
            .capture()?
            .ok_or_else(|| Error::Verification("Claude Code reports no signed-in account after switching".into()))?;
        if live.identity.key != expected.identity.key || !expected.same_primary_data(&live) {
            return Err(Error::Verification(format!(
                "Claude Code configuration holds {} instead of {}",
                live.identity.short(),
                expected.identity.short()
            )));
        }
        let Some(bin) = self.binary() else {
            return Ok("Configuration verified. The claude command was not found, so the CLI check was skipped.".into());
        };
        let out = process::run_capture(&bin, &["auth", "status", "--json"], &[], Duration::from_secs(20))?;
        let v: serde_json::Value = serde_json::from_str(out.trim())
            .map_err(|_| Error::Verification("claude auth status returned unreadable output".into()))?;
        if v.get("loggedIn").and_then(|b| b.as_bool()) != Some(true) {
            return Err(Error::Verification("claude auth status reports that no account is signed in".into()));
        }
        let cli_email = v.get("email").and_then(|e| e.as_str()).map(str::to_lowercase);
        match (cli_email, expected.identity.email.as_deref().map(str::to_lowercase)) {
            (Some(a), Some(b)) if a != b => {
                Err(Error::Verification(format!("claude auth status reports {a}, expected {b}")))
            }
            _ => Ok("Verified with claude auth status".into()),
        }
    }

    fn login(&self, cancel: &AtomicBool, out: OutputSink) -> Result<Snapshot> {
        let bin = self.binary().ok_or_else(|| Error::NotFound("The claude command was not found".into()))?;
        let dir = scratch_dir(&self.ctx, "claude-login")?;
        let isolated_service = {
            use sha2::{Digest, Sha256};
            let d = dir.to_string_lossy();
            format!("{KEYCHAIN_SERVICE}-{}", &hex::encode(Sha256::digest(d.as_bytes()))[..8])
        };
        let cleanup = || {
            if self.use_keychain {
                let _ = self.ctx.io.secrets.delete(&isolated_service, &username());
            }
            let _ = std::fs::remove_dir_all(&dir);
        };
        out("Opening the Claude sign-in page in your browser…");
        if let Err(e) =
            run_login(&bin, &["auth", "login"], &[("CLAUDE_CONFIG_DIR", &dir)], cancel, out, Duration::from_secs(600))
        {
            cleanup();
            return Err(e);
        }
        let creds = (|| -> Result<Option<Vec<u8>>> {
            if self.use_keychain {
                if let Some(v) = self.ctx.io.secrets.get(&isolated_service, &username())? {
                    return Ok(Some(v));
                }
            }
            altflip_core::fsutil::read_optional(&dir.join(".credentials.json"))
        })();
        let account =
            self.ctx.io.read(&Locator::JsonKey { path: dir.join(".claude.json"), key: "oauthAccount".into() });
        cleanup();
        let creds =
            creds?.ok_or_else(|| Error::Credential("Sign-in finished but no Claude credentials were saved".into()))?;
        let primary = if self.use_keychain { self.keychain() } else { self.cred_file() };
        let mut snap = Snapshot::new(
            ProviderId::Claude,
            Identity::default(),
            vec![
                Resource { locator: primary, role: Role::Primary, data: Some(creds) },
                Resource { locator: self.account_key(), role: Role::Primary, data: account? },
            ],
        );
        snap.identity = self
            .identify(&snap)?
            .ok_or_else(|| Error::Credential("Could not read the new Claude account identity".into()))?;
        Ok(snap)
    }
}

static BACKOFF: std::sync::Mutex<Option<std::collections::HashMap<String, chrono::DateTime<chrono::Utc>>>> =
    std::sync::Mutex::new(None);
static VERIFIED: std::sync::Mutex<Option<std::collections::HashMap<String, String>>> = std::sync::Mutex::new(None);

fn token_key(token: &str) -> String {
    altflip_core::short_hash(token.as_bytes())
}

fn backoff_until(token: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let map = BACKOFF.lock().ok()?;
    map.as_ref()?.get(&token_key(token)).copied().filter(|t| *t > chrono::Utc::now())
}

fn set_backoff(token: &str, secs: i64) {
    if let Ok(mut map) = BACKOFF.lock() {
        map.get_or_insert_with(Default::default)
            .insert(token_key(token), chrono::Utc::now() + chrono::Duration::seconds(secs.clamp(60, 3600)));
    }
}

fn rate_limited_text(until: chrono::DateTime<chrono::Utc>) -> String {
    let local = until.with_timezone(&chrono::Local).format("%H:%M");
    format!("Anthropic is limiting usage checks right now. AltFlip shows the last reading and tries again at {local}.")
}

fn anthropic_get(path: &str, token: &str) -> Result<serde_json::Value> {
    if let Some(until) = backoff_until(token) {
        return Err(Error::Platform(rate_limited_text(until)));
    }
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).http_status_as_error(false).build(),
    );
    let mut resp = agent
        .get(&format!("https://api.anthropic.com{path}"))
        .header("Authorization", &format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .header("User-Agent", "AltFlip")
        .call()
        .map_err(|e| Error::Platform(format!("No connection to Anthropic: {e}")))?;
    let status = resp.status().as_u16();
    if status == 429 {
        let secs = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(300);
        set_backoff(token, secs);
        return Err(Error::Platform(rate_limited_text(backoff_until(token).unwrap_or_else(chrono::Utc::now))));
    }
    if status == 401 || status == 403 {
        return Err(Error::Credential(
            "The saved Claude login has expired. Use this account in Claude Code to renew it.".into(),
        ));
    }
    if status != 200 {
        return Err(Error::Platform(format!("Anthropic answered {status}")));
    }
    resp.body_mut()
        .read_json::<serde_json::Value>()
        .map_err(|e| Error::Platform(format!("Unexpected answer from Anthropic: {e}")))
}

pub(crate) fn parse_exact_usage(v: &serde_json::Value, observed_at: &str) -> Vec<UsageReading> {
    let mut out = Vec::new();
    for (key, label, mins) in [
        ("five_hour", "5-hour usage", 300),
        ("seven_day", "Weekly usage", 10080),
        ("seven_day_opus", "Weekly Opus usage", 10080),
    ] {
        let Some(w) = v.get(key).filter(|w| !w.is_null()) else { continue };
        let Some(pct) = w.get("utilization").and_then(|u| u.as_f64()) else { continue };
        out.push(UsageReading {
            usage_type: key.into(),
            label: label.into(),
            used_percent: Some(pct.clamp(0.0, 100.0)),
            used_value: None,
            limit_value: None,
            window_minutes: Some(mins),
            reset_at: w.get("resets_at").and_then(|r| r.as_str()).map(str::to_string),
            source: "Anthropic".into(),
            confidence: "provider-reported".into(),
            observed_at: observed_at.to_string(),
            account_key: None,
        });
    }
    out
}

const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

fn creds_resource(snap: &Snapshot) -> Option<&Resource> {
    snap.primary().find(|r| !matches!(r.locator, Locator::JsonKey { .. }))
}

pub fn needs_renewal(snap: &Snapshot) -> bool {
    creds_resource(snap)
        .and_then(|r| json(r.data.as_deref()))
        .and_then(|c| c.pointer("/claudeAiOauth/expiresAt").and_then(|e| e.as_i64()))
        .is_some_and(|e| e < chrono::Utc::now().timestamp_millis() + 120_000)
}

pub fn creds_locator(snap: &Snapshot) -> Option<Locator> {
    creds_resource(snap).map(|r| r.locator.clone())
}

pub(crate) fn apply_renewal(creds: &[u8], answer: &serde_json::Value, now_ms: i64) -> Result<Vec<u8>> {
    let mut doc: serde_json::Value =
        serde_json::from_slice(creds).map_err(|_| Error::Credential("The saved Claude login is unreadable".into()))?;
    let access = answer
        .get("access_token")
        .and_then(|a| a.as_str())
        .filter(|a| !a.is_empty())
        .ok_or_else(|| Error::Credential("Anthropic did not return a new login".into()))?;
    let o = doc
        .get_mut("claudeAiOauth")
        .and_then(|o| o.as_object_mut())
        .ok_or_else(|| Error::Credential("Not a Claude subscription login".into()))?;
    o.insert("accessToken".into(), access.into());
    if let Some(r) = answer.get("refresh_token").and_then(|r| r.as_str()).filter(|r| !r.is_empty()) {
        o.insert("refreshToken".into(), r.into());
    }
    if let Some(secs) = answer.get("expires_in").and_then(|s| s.as_i64()) {
        o.insert("expiresAt".into(), (now_ms + secs * 1000).into());
    }
    Ok(serde_json::to_vec(&doc)?)
}

pub fn renew(snap: &Snapshot) -> Result<Snapshot> {
    let res = creds_resource(snap).ok_or_else(|| Error::Credential("No saved Claude login".into()))?;
    let creds = res.data.clone().ok_or_else(|| Error::Credential("No saved Claude login".into()))?;
    let refresh = json(Some(&creds))
        .and_then(|c| c.pointer("/claudeAiOauth/refreshToken").and_then(|r| r.as_str()).map(str::to_string))
        .filter(|r| !r.is_empty())
        .ok_or_else(|| Error::Credential("The saved Claude login cannot be renewed. Sign in to it again.".into()))?;
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).http_status_as_error(false).build(),
    );
    let mut resp = agent
        .post("https://console.anthropic.com/v1/oauth/token")
        .header("User-Agent", "AltFlip")
        .send_json(
            serde_json::json!({"grant_type": "refresh_token", "refresh_token": refresh, "client_id": OAUTH_CLIENT_ID}),
        )
        .map_err(|e| Error::Platform(format!("No connection to Anthropic: {e}")))?;
    let status = resp.status().as_u16();
    if status == 400 || status == 401 || status == 403 {
        return Err(Error::Credential("The saved Claude login was signed out. Sign in to this account again.".into()));
    }
    if status != 200 {
        return Err(Error::Platform(format!("Anthropic answered {status} while renewing the login")));
    }
    let answer: serde_json::Value =
        resp.body_mut().read_json().map_err(|e| Error::Platform(format!("Unexpected answer from Anthropic: {e}")))?;
    let updated = apply_renewal(&creds, &answer, chrono::Utc::now().timestamp_millis())?;
    let mut out = snap.clone();
    for r in out.resources.iter_mut() {
        if r.locator == res.locator {
            r.data = Some(updated.clone());
        }
    }
    Ok(out)
}

pub fn exact_usage(snap: &Snapshot) -> Result<Vec<UsageReading>> {
    let creds = creds_resource(snap)
        .and_then(|r| json(r.data.as_deref()))
        .ok_or_else(|| Error::Credential("No saved Claude login".into()))?;
    let oauth =
        creds.get("claudeAiOauth").ok_or_else(|| Error::Unsupported("Not a Claude subscription login".into()))?;
    let token = oauth
        .get("accessToken")
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Error::Credential("No saved Claude login".into()))?;
    if oauth
        .get("expiresAt")
        .and_then(|e| e.as_i64())
        .is_some_and(|e| e < chrono::Utc::now().timestamp_millis() + 60_000)
    {
        return Err(Error::Credential("The saved Claude login has expired".into()));
    }
    let known = VERIFIED.lock().ok().and_then(|m| m.as_ref().and_then(|m| m.get(&token_key(token)).cloned()));
    if known.as_deref() != Some(snap.identity.key.as_str()) {
        let profile = anthropic_get("/api/oauth/profile", token)?;
        let uuid = profile.pointer("/account/uuid").and_then(|u| u.as_str()).unwrap_or_default();
        if uuid.is_empty() || !snap.identity.key.starts_with(&format!("claude:{uuid}:")) {
            return Err(Error::Verification("The saved login belongs to a different Claude account".into()));
        }
        if let Ok(mut m) = VERIFIED.lock() {
            m.get_or_insert_with(Default::default).insert(token_key(token), snap.identity.key.clone());
        }
    }
    let usage = anthropic_get("/api/oauth/usage", token)?;
    let out = parse_exact_usage(&usage, &altflip_core::now_rfc3339());
    if out.is_empty() {
        return Err(Error::Platform("Anthropic returned no usage windows".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::ctx;

    fn seed(ctx: &Ctx, email: &str, uuid: &str, token: &str) {
        let cfg = ctx.home.join(".claude.json");
        let mut doc: serde_json::Value = std::fs::read(&cfg)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(serde_json::json!({"projects": {"/x": {"history": [1]}}}));
        doc["oauthAccount"] = serde_json::json!({"accountUuid": uuid, "emailAddress": email, "organizationUuid": "org", "organizationName": "Org"});
        std::fs::write(&cfg, serde_json::to_vec(&doc).unwrap()).unwrap();
        std::fs::create_dir_all(ctx.home.join(".claude")).unwrap();
        std::fs::write(
            ctx.home.join(".claude/.credentials.json"),
            serde_json::json!({"claudeAiOauth": {"accessToken": token, "refreshToken": format!("r-{token}"), "expiresAt": 1, "subscriptionType": "max"}}).to_string(),
        )
        .unwrap();
    }

    #[test]
    fn capture_apply_switches_identity_and_keeps_other_config() {
        let (_d, ctx) = ctx();
        let c = Claude::with_keychain(ctx.clone(), false);
        assert!(c.capture().unwrap().is_none());
        seed(&ctx, "a@x.dev", "u1", "ta");
        let a = c.capture().unwrap().unwrap();
        assert_eq!(a.identity.email.as_deref(), Some("a@x.dev"));
        assert_eq!(a.identity.plan.as_deref(), Some("max"));
        assert_eq!(c.health(&a), AuthHealth::Ok);
        seed(&ctx, "b@x.dev", "u2", "tb");
        let b = c.capture().unwrap().unwrap();
        assert_ne!(a.identity.key, b.identity.key);
        c.apply(&a).unwrap();
        let live = c.capture().unwrap().unwrap();
        assert_eq!(live.identity.key, a.identity.key);
        assert!(a.same_primary_data(&live));
        let doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(ctx.home.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(doc["projects"]["/x"]["history"][0], 1);
    }

    #[test]
    fn parses_exact_usage_windows() {
        let v = serde_json::json!({"five_hour": {"utilization": 42.5, "resets_at": "2026-11-16T10:00:00Z"}, "seven_day": {"utilization": 63}, "seven_day_opus": null});
        let u = parse_exact_usage(&v, "2026-11-16T08:00:00Z");
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].used_percent, Some(42.5));
        assert_eq!(u[0].reset_at.as_deref(), Some("2026-11-16T10:00:00Z"));
        assert_eq!(u[1].label, "Weekly usage");
    }

    #[test]
    fn renewal_keeps_other_fields_and_updates_expiry() {
        let creds = br#"{"claudeAiOauth":{"accessToken":"old","refreshToken":"r1","expiresAt":1,"subscriptionType":"max","scopes":["a"]},"other":1}"#;
        let out = apply_renewal(
            creds,
            &serde_json::json!({"access_token":"new","refresh_token":"r2","expires_in":3600}),
            1000,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["claudeAiOauth"]["accessToken"], "new");
        assert_eq!(v["claudeAiOauth"]["refreshToken"], "r2");
        assert_eq!(v["claudeAiOauth"]["expiresAt"], 3_601_000);
        assert_eq!(v["claudeAiOauth"]["subscriptionType"], "max");
        assert_eq!(v["other"], 1);
        assert!(apply_renewal(creds, &serde_json::json!({}), 0).is_err());
    }

    #[test]
    fn expired_login_is_not_sent() {
        let (_d, ctx) = ctx();
        let c = Claude::with_keychain(ctx.clone(), false);
        seed(&ctx, "a@x.dev", "u1", "ta");
        let s = c.capture().unwrap().unwrap();
        assert!(matches!(exact_usage(&s), Err(Error::Credential(_))));
    }

    #[test]
    fn keychain_mode_prefers_keychain_then_file() {
        let (_d, ctx) = ctx();
        let c = Claude::with_keychain(ctx.clone(), true);
        seed(&ctx, "a@x.dev", "u1", "ta");
        let s = c.capture().unwrap().unwrap();
        assert_eq!(s.primary().filter(|r| matches!(r.locator, Locator::File { .. })).count(), 1);
        ctx.io.write(&c.keychain(), br#"{"claudeAiOauth":{"refreshToken":"k"}}"#).unwrap();
        let s = c.capture().unwrap().unwrap();
        assert!(s.primary().any(|r| matches!(r.locator, Locator::Credential { .. })));
    }
}
