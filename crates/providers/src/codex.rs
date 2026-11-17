use crate::{json, run_login, scratch_dir, version_of, Ctx, Detection, OutputSink, Provider};
use altflip_core::process::{self, ProcView};
use altflip_core::resource::{Locator, Resource, Role, Snapshot};
use altflip_core::{
    jwt, short_hash, AuthHealth, Capabilities, Compatibility, Error, Identity, ProcessInfo, ProcessKind, ProviderId,
    Result, Support, UsageReading,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime};

pub struct Codex {
    ctx: Ctx,
}

#[derive(Debug, PartialEq, Eq)]
enum Store {
    File,
    Keyring,
    Auto,
}

fn classify(p: &ProcView) -> Option<ProcessKind> {
    let exe = p.exe.as_ref().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    if exe.contains("Codex.app") {
        return Some(if p.name == "Codex" { ProcessKind::App } else { ProcessKind::Helper });
    }
    let name = p.name.to_lowercase();
    if name == "codex" || name == "codex.exe" {
        return Some(ProcessKind::Cli);
    }
    if (name == "node" || name == "node.exe") && p.cmd.iter().any(|a| a.contains("@openai/codex")) {
        return Some(ProcessKind::Cli);
    }
    None
}

impl Codex {
    pub fn new(ctx: Ctx) -> Self {
        Codex { ctx }
    }

    fn home(&self) -> PathBuf {
        crate::env_dir_for(&self.ctx.home, "CODEX_HOME").unwrap_or_else(|| self.ctx.home.join(".codex"))
    }

    fn auth(&self) -> Locator {
        Locator::File { path: self.home().join("auth.json") }
    }

    fn store(&self) -> Store {
        let Ok(cfg) = std::fs::read_to_string(self.home().join("config.toml")) else { return Store::File };
        for line in cfg.lines() {
            let l = line.trim();
            if l.starts_with('[') {
                break;
            }
            if let Some(v) = l.strip_prefix("cli_auth_credentials_store") {
                let v = v.trim_start().trim_start_matches('=').trim().trim_matches('"').trim_matches('\'');
                return match v {
                    "keyring" => Store::Keyring,
                    "auto" => Store::Auto,
                    _ => Store::File,
                };
            }
        }
        Store::File
    }

    fn binary(&self) -> Option<PathBuf> {
        process::find_binary("codex", &[])
    }

    fn newest_rollouts(&self, limit: usize) -> Vec<PathBuf> {
        let root = self.home().join("sessions");
        let cutoff = SystemTime::now() - Duration::from_secs(8 * 86_400);
        let mut files: Vec<(SystemTime, PathBuf)> = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let Ok(meta) = e.metadata() else { continue };
                if meta.is_dir() {
                    stack.push(p);
                } else if p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("rollout-"))
                    && p.extension().is_some_and(|x| x == "jsonl")
                {
                    if let Ok(m) = meta.modified() {
                        if m > cutoff {
                            files.push((m, p));
                        }
                    }
                }
            }
        }
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        files.into_iter().take(limit).map(|f| f.1).collect()
    }
}

pub(crate) struct RolloutReading {
    pub observed_at: String,
    pub account_key: Option<String>,
    pub rate_limits: serde_json::Value,
}

fn find_key<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    match v {
        serde_json::Value::Object(m) => {
            if let Some(x) = m.get(key).filter(|x| !x.is_null()) {
                return Some(x);
            }
            m.values().find_map(|x| find_key(x, key))
        }
        serde_json::Value::Array(a) => a.iter().find_map(|x| find_key(x, key)),
        _ => None,
    }
}

fn session_owner(path: &Path) -> Option<String> {
    use std::io::{BufRead, BufReader, Read};
    let f = std::fs::File::open(path).ok()?;
    let mut first = String::new();
    BufReader::new(f).take(256 * 1024).read_line(&mut first).ok()?;
    let v: serde_json::Value = serde_json::from_str(first.trim()).ok()?;
    let meta = v.get("payload").filter(|_| v.get("type").and_then(|t| t.as_str()) == Some("session_meta"))?;
    let user = meta.get("creator_user_id").and_then(|u| u.as_str()).filter(|u| !u.is_empty())?;
    let account = meta.get("creator_account_id").and_then(|a| a.as_str()).unwrap_or_default();
    Some(format!("chatgpt:{user}:{account}"))
}

#[cfg(test)]
pub(crate) fn rate_limits_from(path: &Path) -> Option<RolloutReading> {
    rate_limits_owned(path, session_owner(path))
}

fn rate_limits_owned(path: &Path, account_key: Option<String>) -> Option<RolloutReading> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let size = f.metadata().ok()?.len();
    let start = size.saturating_sub(600_000);
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    for line in text.lines().rev() {
        if !line.contains("\"rate_limits\"") {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let Some(rl) = find_key(&v, "rate_limits") else { continue };
        if rl.get("primary").is_none_or(|p| p.is_null()) {
            continue;
        }
        let observed_at = v.get("timestamp").and_then(|t| t.as_str()).map(str::to_string).or_else(|| mtime.clone())?;
        return Some(RolloutReading { observed_at, account_key, rate_limits: rl.clone() });
    }
    None
}

pub(crate) fn readings_from(r: &RolloutReading) -> Vec<UsageReading> {
    let now = chrono::Utc::now();
    let stamp =
        chrono::DateTime::parse_from_rfc3339(&r.observed_at).map(|d| d.with_timezone(&chrono::Utc)).unwrap_or(now);
    let mut out = Vec::new();
    for slot in ["primary", "secondary"] {
        let Some(w) = r.rate_limits.get(slot).filter(|w| !w.is_null()) else { continue };
        let Some(mut pct) = w.get("used_percent").and_then(|u| u.as_f64()) else { continue };
        let mins =
            w.get("window_minutes").and_then(|m| m.as_i64()).unwrap_or(if slot == "primary" { 300 } else { 10080 });
        let mut reset =
            w.get("resets_at").and_then(|r| r.as_i64()).and_then(|s| chrono::DateTime::from_timestamp(s, 0)).or_else(
                || w.get("resets_in_seconds").and_then(|s| s.as_i64()).map(|s| stamp + chrono::Duration::seconds(s)),
            );
        if reset.is_some_and(|t| t < now) {
            pct = 0.0;
            reset = None;
        }
        out.push(UsageReading {
            usage_type: slot.into(),
            label: window_label(mins),
            used_percent: Some(pct),
            used_value: None,
            limit_value: None,
            window_minutes: Some(mins),
            reset_at: reset.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            source: "Codex session log".into(),
            confidence: "provider-reported".into(),
            observed_at: r.observed_at.clone(),
            account_key: r.account_key.clone(),
        });
    }
    out
}

fn window_label(mins: i64) -> String {
    match mins {
        300 => "5-hour usage".into(),
        10080 => "Weekly usage".into(),
        m if m % 1440 == 0 => format!("{}-day usage", m / 1440),
        m if m % 60 == 0 => format!("{}-hour usage", m / 60),
        m => format!("{m}-minute usage"),
    }
}

impl Provider for Codex {
    fn id(&self) -> ProviderId {
        ProviderId::Codex
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
        let installed = bin.is_some() || self.home().is_dir();
        let auth_exists = self.home().join("auth.json").is_file();
        let (compatibility, note) = match (installed, self.store()) {
            (false, _) => (Compatibility::NotInstalled, None),
            (true, Store::Keyring) => (
                Compatibility::Unsupported,
                Some("Codex is configured to keep credentials in the OS keyring (cli_auth_credentials_store = \"keyring\"). AltFlip currently switches file-based Codex credentials only.".into()),
            ),
            (true, Store::Auto) if !auth_exists => (
                Compatibility::Unsupported,
                Some("Codex credential storage is set to auto and no auth.json exists, so credentials live in the OS keyring.".into()),
            ),
            (true, _) => (Compatibility::Supported, None),
        };
        Detection { installed, version, binary_path: bin, config_path: Some(self.home()), compatibility, note }
    }

    fn locators(&self) -> Vec<(Locator, Role)> {
        vec![(self.auth(), Role::Primary)]
    }

    fn identify(&self, snap: &Snapshot) -> Result<Option<Identity>> {
        let Some(auth) = json(snap.data_for(&self.auth())) else { return Ok(None) };
        let tokens = auth.get("tokens").filter(|t| !t.is_null());
        if let Some(tokens) = tokens {
            let id_token = tokens.get("id_token").and_then(|t| t.as_str()).unwrap_or_default();
            let claims = jwt::claims(id_token).unwrap_or_default();
            let auth_claim = claims.get("https://api.openai.com/auth").cloned().unwrap_or_default();
            let s = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
            let email = s(&claims, "email");
            let account = s(&auth_claim, "chatgpt_account_id").or_else(|| s(tokens, "account_id"));
            let user =
                s(&auth_claim, "chatgpt_user_id").or_else(|| s(&auth_claim, "user_id")).or_else(|| email.clone());
            let key = match (&user, &account) {
                (None, None) => match s(tokens, "refresh_token") {
                    Some(r) => format!("codex-token:{}", short_hash(r.as_bytes())),
                    None => return Ok(None),
                },
                _ => format!("chatgpt:{}:{}", user.unwrap_or_default(), account.unwrap_or_default()),
            };
            return Ok(Some(Identity {
                key,
                email,
                display_name: s(&claims, "name"),
                organization: None,
                plan: s(&auth_claim, "chatgpt_plan_type"),
                auth_method: Some("chatgpt".into()),
            }));
        }
        if let Some(k) = auth.get("OPENAI_API_KEY").and_then(|k| k.as_str()).filter(|k| !k.is_empty()) {
            let tail: String = k.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
            return Ok(Some(Identity {
                key: format!("apikey:{}", short_hash(k.as_bytes())),
                email: None,
                display_name: Some(format!("API key …{tail}")),
                organization: None,
                plan: Some("api".into()),
                auth_method: Some("api-key".into()),
            }));
        }
        Ok(None)
    }

    fn health(&self, snap: &Snapshot) -> AuthHealth {
        let Some(auth) = json(snap.data_for(&self.auth())) else { return AuthHealth::NeedsReauth };
        if let Some(t) = auth.get("tokens").filter(|t| !t.is_null()) {
            return if t.get("refresh_token").and_then(|r| r.as_str()).is_some_and(|r| !r.is_empty()) {
                AuthHealth::Ok
            } else {
                AuthHealth::NeedsReauth
            };
        }
        if auth.get("OPENAI_API_KEY").and_then(|k| k.as_str()).is_some() {
            return AuthHealth::Ok;
        }
        AuthHealth::Unknown
    }

    fn processes(&self) -> Vec<ProcessInfo> {
        process::find(classify)
    }

    fn login(&self, cancel: &AtomicBool, out: OutputSink) -> Result<Snapshot> {
        let bin = self.binary().ok_or_else(|| Error::NotFound("The codex command was not found".into()))?;
        let dir = scratch_dir(&self.ctx, "codex-login")?;
        std::fs::write(dir.join("config.toml"), "cli_auth_credentials_store = \"file\"\n")
            .map_err(|e| Error::io("Cannot prepare the Codex sign-in folder", e))?;
        out("Opening the ChatGPT sign-in page in your browser…");
        let res = run_login(&bin, &["login"], &[("CODEX_HOME", &dir)], cancel, out, Duration::from_secs(600));
        let data = altflip_core::fsutil::read_optional(&dir.join("auth.json"));
        let _ = std::fs::remove_dir_all(&dir);
        res?;
        let data = data?.ok_or_else(|| Error::Credential("Sign-in finished but Codex saved no credentials".into()))?;
        let mut snap = Snapshot::new(
            ProviderId::Codex,
            Identity::default(),
            vec![Resource { locator: self.auth(), role: Role::Primary, data: Some(data) }],
        );
        snap.identity = self
            .identify(&snap)?
            .ok_or_else(|| Error::Credential("Could not read the new Codex account identity".into()))?;
        Ok(snap)
    }

    fn usage(&self) -> Vec<UsageReading> {
        let mut seen: Vec<Option<String>> = Vec::new();
        let mut out = Vec::new();
        for f in self.newest_rollouts(40) {
            let owner = session_owner(&f);
            if seen.contains(&owner) {
                continue;
            }
            let Some(r) = rate_limits_owned(&f, owner) else { continue };
            seen.push(r.account_key.clone());
            out.extend(readings_from(&r));
            if seen.len() >= 6 {
                break;
            }
        }
        out
    }
}

pub(crate) fn parse_live_usage(v: &serde_json::Value, observed_at: &str, now: i64) -> Vec<UsageReading> {
    let mut out = Vec::new();
    let Some(rl) = v.get("rate_limit").filter(|r| !r.is_null()) else { return out };
    for (slot, key) in [("primary", "primary_window"), ("secondary", "secondary_window")] {
        let Some(w) = rl.get(key).filter(|w| !w.is_null()) else { continue };
        let Some(pct) = w.get("used_percent").and_then(|u| u.as_f64()) else { continue };
        let mins = w.get("limit_window_seconds").and_then(|s| s.as_i64()).map(|s| s / 60);
        let reset = w
            .get("reset_at")
            .and_then(|r| r.as_i64())
            .or_else(|| w.get("reset_after_seconds").and_then(|s| s.as_i64()).map(|s| now + s));
        out.push(UsageReading {
            usage_type: slot.into(),
            label: mins.map(window_label).unwrap_or_else(|| "Usage".into()),
            used_percent: Some(pct.clamp(0.0, 100.0)),
            used_value: None,
            limit_value: None,
            window_minutes: mins,
            reset_at: reset
                .and_then(|r| chrono::DateTime::from_timestamp(r, 0))
                .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            source: "OpenAI".into(),
            confidence: "provider-reported".into(),
            observed_at: observed_at.to_string(),
            account_key: None,
        });
    }
    out
}

pub fn live_usage(snap: &Snapshot) -> Result<Vec<UsageReading>> {
    let auth = snap
        .resources
        .iter()
        .find(|r| r.role == Role::Primary)
        .and_then(|r| json(r.data.as_deref()))
        .ok_or_else(|| Error::Credential("No saved Codex login".into()))?;
    let tokens = auth
        .get("tokens")
        .filter(|t| !t.is_null())
        .ok_or_else(|| Error::Unsupported("API-key logins have no ChatGPT plan limits".into()))?;
    let token = tokens
        .get("access_token")
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Error::Credential("No saved Codex login".into()))?;
    let account = tokens.get("account_id").and_then(|t| t.as_str()).unwrap_or_default();
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).http_status_as_error(false).build(),
    );
    let mut req = agent
        .get("https://chatgpt.com/backend-api/wham/usage")
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/json")
        .header("User-Agent", "codex-cli");
    if !account.is_empty() {
        req = req.header("ChatGPT-Account-Id", account);
    }
    let mut resp = req.call().map_err(|e| Error::Platform(format!("No connection to OpenAI: {e}")))?;
    let status = resp.status().as_u16();
    if status == 401 || status == 403 {
        return Err(Error::Credential(
            "The saved Codex login has expired. Switch to this account and use Codex once to renew it.".into(),
        ));
    }
    if status != 200 {
        return Err(Error::Platform(format!("OpenAI answered {status}")));
    }
    let v: serde_json::Value =
        resp.body_mut().read_json().map_err(|e| Error::Platform(format!("Unexpected answer from OpenAI: {e}")))?;
    let out = parse_live_usage(&v, &altflip_core::now_rfc3339(), chrono::Utc::now().timestamp());
    if out.is_empty() {
        return Err(Error::Platform("OpenAI returned no usage windows".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{ctx, jwt};

    fn auth(email: &str, acct: &str) -> String {
        let id = jwt(
            serde_json::json!({"email": email, "https://api.openai.com/auth": {"chatgpt_account_id": acct, "chatgpt_user_id": format!("user-{email}"), "chatgpt_plan_type": "plus"}}),
        );
        serde_json::json!({"auth_mode": "chatgpt", "OPENAI_API_KEY": null, "tokens": {"id_token": id, "access_token": "a", "refresh_token": format!("r-{email}"), "account_id": acct}, "last_refresh": "2026-01-01T00:00:00Z"}).to_string()
    }

    #[test]
    fn identifies_workspaces_separately_and_switches() {
        let (_d, ctx) = ctx();
        std::fs::create_dir_all(ctx.home.join(".codex")).unwrap();
        let c = Codex::new(ctx.clone());
        if std::env::var_os("CODEX_HOME").is_some() {
            return;
        }
        std::fs::write(ctx.home.join(".codex/auth.json"), auth("a@x.dev", "w1")).unwrap();
        let a = c.capture().unwrap().unwrap();
        assert_eq!(a.identity.plan.as_deref(), Some("plus"));
        std::fs::write(ctx.home.join(".codex/auth.json"), auth("a@x.dev", "w2")).unwrap();
        let b = c.capture().unwrap().unwrap();
        assert_ne!(a.identity.key, b.identity.key);
        c.apply(&a).unwrap();
        assert!(c.verify(&a).is_ok());
        assert!(c.verify(&b).is_err());
    }

    #[test]
    fn api_key_identity_masks_key() {
        let (_d, ctx) = ctx();
        if std::env::var_os("CODEX_HOME").is_some() {
            return;
        }
        std::fs::create_dir_all(ctx.home.join(".codex")).unwrap();
        std::fs::write(ctx.home.join(".codex/auth.json"), r#"{"OPENAI_API_KEY":"sk-secret-1234","tokens":null}"#)
            .unwrap();
        let s = Codex::new(ctx).capture().unwrap().unwrap();
        assert_eq!(s.identity.display_name.as_deref(), Some("API key …1234"));
        assert!(!s.identity.key.contains("secret"));
    }

    #[test]
    fn detects_keyring_store_as_unsupported() {
        let (_d, ctx) = ctx();
        if std::env::var_os("CODEX_HOME").is_some() {
            return;
        }
        std::fs::create_dir_all(ctx.home.join(".codex")).unwrap();
        std::fs::write(
            ctx.home.join(".codex/config.toml"),
            "model = \"x\"\ncli_auth_credentials_store = \"keyring\"\n",
        )
        .unwrap();
        assert_eq!(Codex::new(ctx).store(), Store::Keyring);
    }

    #[test]
    fn parses_live_usage_windows() {
        let v = serde_json::json!({"plan_type": "plus", "rate_limit": {"allowed": true, "primary_window": {"used_percent": 3, "limit_window_seconds": 18000, "reset_after_seconds": 100, "reset_at": 1794843293}, "secondary_window": {"used_percent": 25, "limit_window_seconds": 604800, "reset_after_seconds": 50}}});
        let u = parse_live_usage(&v, "2026-11-16T00:00:00Z", 1000);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].used_percent, Some(3.0));
        assert_eq!(u[0].label, "5-hour usage");
        assert_eq!(u[1].label, "Weekly usage");
        assert_eq!(u[1].reset_at.as_deref(), Some("1970-01-01T00:17:30Z"));
        assert_eq!(u[1].source, "OpenAI");
    }

    #[test]
    fn reads_rate_limits_with_owner_from_rollout() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("rollout-x.jsonl");
        let reset = chrono::Utc::now().timestamp() + 3600;
        std::fs::write(
            &p,
            format!(
                "{{\"timestamp\":\"2026-11-01T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{{\"creator_user_id\":\"user-1\",\"creator_account_id\":\"acc-1\"}}}}\n\
                 {{\"timestamp\":\"2026-11-01T00:01:00Z\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"rate_limits\":{{\"primary\":{{\"used_percent\":42.0,\"window_minutes\":300,\"resets_at\":{reset}}},\"secondary\":{{\"used_percent\":80.0,\"window_minutes\":10080,\"resets_at\":1}}}}}}}}\n"
            ),
        )
        .unwrap();
        let r = rate_limits_from(&p).unwrap();
        assert_eq!(r.observed_at, "2026-11-01T00:01:00Z");
        assert_eq!(r.account_key.as_deref(), Some("chatgpt:user-1:acc-1"));
        let u = readings_from(&r);
        assert_eq!(u[0].used_percent, Some(42.0));
        assert_eq!(u[0].label, "5-hour usage");
        assert_eq!(u[1].used_percent, Some(0.0));
        assert!(u[1].reset_at.is_none());
    }
}
