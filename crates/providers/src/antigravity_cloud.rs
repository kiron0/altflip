use crate::antigravity::{decode_payload, parse_quota_summary, refresh_token};
use altflip_core::resource::Snapshot;
use altflip_core::{short_hash, Error, Result, UsageReading};
use base64::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const CLIENT_ID: &str = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const BASES: [&str; 2] = ["https://daily-cloudcode-pa.googleapis.com", "https://cloudcode-pa.googleapis.com"];
const DEFAULT_PROJECT: &str = "aicode-consumers";
const HUB_VERSION: &str = "4.3.0";
const SIGNED_OUT: &str =
    "Google no longer accepts this saved Antigravity sign-in. Switch to it, sign in again in Antigravity, then save it.";

pub struct Cloud {
    pub token_url: String,
    pub bases: Vec<String>,
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CloudQuota {
    pub readings: Vec<UsageReading>,
    pub tier: Option<String>,
}

struct Token {
    access: String,
    until: Instant,
}

struct Project {
    id: Option<String>,
    tier: Option<String>,
    until: Instant,
}

type Cache<T> = Mutex<Option<HashMap<String, T>>>;

static TOKENS: Cache<Token> = Mutex::new(None);
static PROJECTS: Cache<Project> = Mutex::new(None);
static SECRETS: OnceLock<Vec<String>> = OnceLock::new();

pub fn hub_user_agent() -> String {
    let os = match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        _ => "linux",
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => other,
    };
    format!("antigravity/hub/{HUB_VERSION} (os_type={os}; arch={arch})")
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    let last = hay.len() - needle.len();
    let mut i = 0;
    while i <= last {
        let at = i + hay[i..=last].iter().position(|c| *c == needle[0])?;
        if &hay[at..at + needle.len()] == needle {
            return Some(at);
        }
        i = at + 1;
    }
    None
}

fn token_end(b: &[u8], start: usize) -> usize {
    b[start..]
        .iter()
        .position(|c| !(c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_'))
        .map_or(b.len(), |e| start + e)
}

pub fn secrets_in(bytes: &[u8]) -> Vec<String> {
    let anchor = find(bytes, CLIENT_ID.as_bytes());
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut i = 0;
    while let Some(off) = find(&bytes[i..], b"GOCSPX-") {
        let start = i + off;
        let end = token_end(bytes, start);
        if end - start >= 30 {
            found.push((start, String::from_utf8_lossy(&bytes[start..end]).into_owned()));
        }
        i = end.max(start + 1);
    }
    if let Some(a) = anchor {
        found.sort_by_key(|(s, _)| if *s > a { s - a } else { usize::MAX / 2 + (a - s) });
    }
    let mut out: Vec<String> = Vec::new();
    for (_, s) in found {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

fn installed_sources(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    #[cfg(target_os = "macos")]
    for base in [PathBuf::from("/Applications"), home.join("Applications")] {
        for name in ["Antigravity IDE.app", "Antigravity.app"] {
            out.push(base.join(name).join("Contents/Resources/app/out/main.js"));
        }
    }
    #[cfg(windows)]
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        for dir in ["Antigravity IDE", "Antigravity"] {
            out.push(local.join("Programs").join(dir).join("resources/app/out/main.js"));
        }
    }
    #[cfg(target_os = "linux")]
    for dir in ["/usr/share/antigravity", "/opt/Antigravity"] {
        out.push(PathBuf::from(dir).join("resources/app/out/main.js"));
    }
    out.push(home.join(".gemini/bin").join(if cfg!(windows) { "agy.exe" } else { "agy" }));
    out
}

pub fn installed_secrets(home: &Path) -> Vec<String> {
    SECRETS
        .get_or_init(|| {
            for path in installed_sources(home) {
                let Ok(bytes) = std::fs::read(&path) else { continue };
                if find(&bytes, CLIENT_ID.as_bytes()).is_none() {
                    continue;
                }
                let found = secrets_in(&bytes);
                if !found.is_empty() {
                    return found;
                }
            }
            Vec::new()
        })
        .clone()
}

fn b64_runs(b: &[u8]) -> Vec<Vec<u8>> {
    let is_b64 = |c: u8| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'=' | b'-' | b'_');
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !is_b64(b[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && is_b64(b[i]) {
            i += 1;
        }
        if i - start >= 40 {
            let run = std::str::from_utf8(&b[start..i]).unwrap_or_default().trim_end_matches('=');
            let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
                .decode(run)
                .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(run));
            if let Ok(d) = decoded {
                out.push(d);
            }
        }
    }
    out
}

fn scan_refresh_token(b: &[u8], depth: u8) -> Option<String> {
    let mut i = 0;
    while let Some(off) = find(&b[i..], b"1//") {
        let start = i + off;
        let end = token_end(b, start + 3);
        if end - start >= 30 {
            return Some(String::from_utf8_lossy(&b[start..end]).into_owned());
        }
        i = start + 3;
    }
    if depth >= 3 {
        return None;
    }
    b64_runs(b).iter().find_map(|d| scan_refresh_token(d, depth + 1))
}

pub fn refresh_token_in(data: &[&[u8]]) -> Option<String> {
    data.iter()
        .find_map(|d| decode_payload(d).as_ref().and_then(refresh_token).map(str::to_string))
        .or_else(|| data.iter().find_map(|d| scan_refresh_token(d, 0)))
}

pub fn saved_refresh_token(snap: &Snapshot) -> Option<String> {
    let data: Vec<&[u8]> = snap.resources.iter().filter_map(|r| r.data.as_deref()).collect();
    refresh_token_in(&data)
}

fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).http_status_as_error(false).build(),
    )
}

fn offline(e: impl std::fmt::Display) -> Error {
    Error::Platform(format!("No connection to Google: {e}"))
}

fn cached<T, R>(cache: &Cache<T>, key: &str, pick: impl Fn(&T) -> Option<R>) -> Option<R> {
    cache.lock().ok()?.as_ref()?.get(key).and_then(pick)
}

fn remember<T>(cache: &Cache<T>, key: &str, value: T) {
    if let Ok(mut m) = cache.lock() {
        m.get_or_insert_with(HashMap::new).insert(key.to_string(), value);
    }
}

fn access_token(cloud: &Cloud, key: &str, refresh: &str) -> Result<String> {
    if let Some(t) = cached(&TOKENS, key, |t| (t.until > Instant::now()).then(|| t.access.clone())) {
        return Ok(t);
    }
    if cloud.secrets.is_empty() {
        return Err(Error::Unsupported(
            "Live quota for saved accounts needs Antigravity or agy installed on this computer".into(),
        ));
    }
    let ua = hub_user_agent();
    let mut last = Error::Platform("Google did not renew the sign-in".into());
    for secret in &cloud.secrets {
        let mut resp = agent()
            .post(&cloud.token_url)
            .header("User-Agent", &ua)
            .send_form([
                ("client_id", CLIENT_ID),
                ("client_secret", secret.as_str()),
                ("refresh_token", refresh),
                ("grant_type", "refresh_token"),
            ])
            .map_err(offline)?;
        let status = resp.status().as_u16();
        let body: serde_json::Value = resp.body_mut().read_json().unwrap_or_default();
        if status == 200 {
            let access = body
                .get("access_token")
                .and_then(|a| a.as_str())
                .filter(|a| !a.is_empty())
                .ok_or_else(|| Error::Platform("Google renewed the sign-in without a token".into()))?
                .to_string();
            let secs = body.get("expires_in").and_then(|e| e.as_u64()).unwrap_or(3600).saturating_sub(300).max(60);
            remember(&TOKENS, key, Token { access: access.clone(), until: Instant::now() + Duration::from_secs(secs) });
            return Ok(access);
        }
        let code = body.get("error").and_then(|e| e.as_str()).unwrap_or_default();
        match code {
            "invalid_client" | "unauthorized_client" => {
                last = Error::Platform("Google rejected the Antigravity client".into());
            }
            "invalid_grant" => return Err(Error::Credential(SIGNED_OUT.into())),
            _ => return Err(Error::Platform(format!("Google answered {status} {code}").trim().to_string())),
        }
    }
    Err(last)
}

fn label(t: &serde_json::Value) -> Option<String> {
    ["name", "id"]
        .iter()
        .find_map(|k| t.get(*k).and_then(|v| v.as_str()).map(str::trim).filter(|v| !v.is_empty()))
        .map(str::to_string)
}

pub fn subscription_tier(v: &serde_json::Value) -> Option<String> {
    if let Some(paid) = v.get("paidTier").and_then(label) {
        return Some(paid);
    }
    let restricted = v.get("ineligibleTiers").and_then(|t| t.as_array()).is_some_and(|t| !t.is_empty());
    let default_allowed = v
        .get("allowedTiers")
        .and_then(|t| t.as_array())
        .and_then(|t| t.iter().find(|x| x.get("isDefault").and_then(|d| d.as_bool()) == Some(true)))
        .and_then(label);
    let current = v.get("currentTier").and_then(label);
    if restricted {
        default_allowed.or(current).map(|l| format!("{l} (Restricted)"))
    } else {
        current.or(default_allowed)
    }
}

fn post(base: &str, method: &str, token: &str, ua: &str, body: &serde_json::Value) -> Result<(u16, serde_json::Value)> {
    let mut resp = agent()
        .post(&format!("{base}/v1internal:{method}"))
        .header("Authorization", &format!("Bearer {token}"))
        .header("User-Agent", ua)
        .send_json(body)
        .map_err(offline)?;
    let status = resp.status().as_u16();
    let v = resp.body_mut().read_json().unwrap_or_default();
    Ok((status, v))
}

fn project(cloud: &Cloud, key: &str, token: &str) -> (Option<String>, Option<String>) {
    if let Some(p) = cached(&PROJECTS, key, |p| (p.until > Instant::now()).then(|| (p.id.clone(), p.tier.clone()))) {
        return p;
    }
    let ua = hub_user_agent();
    let meta = serde_json::json!({"metadata": {"ideType": "ANTIGRAVITY"}});
    for base in &cloud.bases {
        let Ok((200, v)) = post(base, "loadCodeAssist", token, &ua, &meta) else { continue };
        let id =
            v.get("cloudaicompanionProject").and_then(|p| p.as_str()).filter(|p| !p.is_empty()).map(str::to_string);
        let tier = subscription_tier(&v);
        let until = Instant::now() + Duration::from_secs(6 * 3600);
        remember(&PROJECTS, key, Project { id: id.clone(), tier: tier.clone(), until });
        return (id, tier);
    }
    (None, None)
}

fn buckets(v: &serde_json::Value) -> impl Iterator<Item = &serde_json::Value> {
    v.get("groups")
        .and_then(|g| g.as_array())
        .into_iter()
        .flatten()
        .flat_map(|g| g.get("buckets").and_then(|b| b.as_array()).into_iter().flatten())
}

fn has_gemini(v: &serde_json::Value) -> bool {
    buckets(v).any(|b| {
        b.get("bucketId").and_then(|i| i.as_str()).is_some_and(|i| i.to_ascii_lowercase().starts_with("gemini"))
    })
}

fn quota_summary(cloud: &Cloud, token: &str, project: Option<&str>) -> Result<serde_json::Value> {
    let ua = hub_user_agent();
    let primary = serde_json::json!({"project": project.unwrap_or(DEFAULT_PROJECT)});
    let attempts = [(ua.as_str(), primary.clone()), ("antigravity", primary), (ua.as_str(), serde_json::json!({}))];
    let mut partial: Option<serde_json::Value> = None;
    let mut last = Error::Platform("Google returned no quota".into());
    for base in &cloud.bases {
        for (i, (agent_ua, body)) in attempts.iter().enumerate() {
            let (status, v) = post(base, "retrieveUserQuotaSummary", token, agent_ua, body)?;
            if status == 401 {
                return Err(Error::Credential(SIGNED_OUT.into()));
            }
            if status == 403 && i + 1 < attempts.len() {
                continue;
            }
            if status != 200 {
                last = Error::Platform(format!("Google answered {status}"));
                break;
            }
            if buckets(&v).next().is_none() {
                break;
            }
            if has_gemini(&v) {
                return Ok(v);
            }
            partial.get_or_insert(v);
            if *agent_ua == "antigravity" && i + 1 < attempts.len() {
                continue;
            }
            break;
        }
    }
    partial.ok_or(last)
}

pub fn fetch(cloud: &Cloud, refresh: &str, now: chrono::DateTime<chrono::Utc>) -> Result<CloudQuota> {
    let key = short_hash(refresh.as_bytes());
    let token = access_token(cloud, &key, refresh)?;
    let (project_id, tier) = project(cloud, &key, &token);
    let v = quota_summary(cloud, &token, project_id.as_deref())?;
    let readings = parse_quota_summary(&serde_json::json!({"response": v}), "", now);
    if readings.is_empty() {
        return Err(Error::Platform("Google returned no quota windows for this account".into()));
    }
    Ok(CloudQuota { readings, tier })
}

pub fn usage(snap: &Snapshot, home: &Path) -> Result<CloudQuota> {
    let refresh = saved_refresh_token(snap).ok_or_else(|| Error::Credential(SIGNED_OUT.into()))?;
    let cloud = Cloud {
        token_url: TOKEN_URL.into(),
        bases: BASES.iter().map(|b| b.to_string()).collect(),
        secrets: installed_secrets(home),
    };
    fetch(&cloud, &refresh, chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    type Route = (&'static str, u16, &'static str);

    fn serve(routes: Vec<Route>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for (path, status, body) in routes {
                let (mut s, _) = l.accept().unwrap();
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                let mut len = 0usize;
                let mut ua = String::new();
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    let lower = h.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                    if lower.starts_with("user-agent:") {
                        ua = h[11..].trim().to_string();
                    }
                    if h.trim().is_empty() {
                        break;
                    }
                }
                let mut buf = vec![0; len];
                r.read_exact(&mut buf).unwrap();
                assert!(line.contains(path), "{line} should hit {path}");
                let target = line.split_whitespace().nth(1).unwrap_or_default().to_string();
                seen.push(format!("{target} | {ua} | {}", String::from_utf8_lossy(&buf)));
                write!(
                    s,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
            seen
        });
        (url, h)
    }

    const SUMMARY: &str = r#"{"groups":[{"displayName":"Gemini Models","buckets":[{"bucketId":"gemini-5h","window":"5h","remainingFraction":0.75,"resetTime":"2030-01-01T05:00:00Z"},{"bucketId":"gemini-weekly","window":"weekly","resetTime":"2030-01-07T00:00:00Z"}]},{"displayName":"Claude and GPT models","buckets":[{"bucketId":"3p-weekly","window":"weekly","remainingFraction":0.1,"resetTime":"2030-01-07T00:00:00Z"}]}]}"#;

    #[test]
    fn finds_paired_secret_first() {
        let blob = format!(
            "x=\"GOCSPX-other00000000000000000000000\";a=\"{CLIENT_ID}\",b=\"GOCSPX-right00000000000000000000000\";"
        );
        assert_eq!(
            secrets_in(blob.as_bytes()),
            vec!["GOCSPX-right00000000000000000000000", "GOCSPX-other00000000000000000000000"]
        );
    }

    #[test]
    fn finds_refresh_token_in_nested_ide_state() {
        let inner = base64::engine::general_purpose::STANDARD
            .encode(b"\x0a\x05ya29.\x12\x241//0abcdefghijklmnopqrstuvwxyzABCDEFG\x18\x01");
        let outer = base64::engine::general_purpose::STANDARD.encode(format!("\x0a\x0eoauthTokenInfo\x12\x01{inner}"));
        let other = b"go-keyring-base64:not-json".to_vec();
        assert_eq!(
            refresh_token_in(&[other.as_slice(), outer.as_bytes()]).as_deref(),
            Some("1//0abcdefghijklmnopqrstuvwxyzABCDEFG")
        );
    }

    #[test]
    fn prefers_keychain_json_token() {
        let json = r#"{"token":{"access_token":"a","refresh_token":"1//0fromjson-xxxxxxxxxxxxxxxxxxxxxx"}}"#;
        let ide = b"1//0fromide-yyyyyyyyyyyyyyyyyyyyyyyyyy";
        assert_eq!(
            refresh_token_in(&[ide.as_slice(), json.as_bytes()]).as_deref(),
            Some("1//0fromjson-xxxxxxxxxxxxxxxxxxxxxx")
        );
    }

    #[test]
    fn tier_prefers_paid_and_marks_restricted() {
        assert_eq!(
            subscription_tier(
                &serde_json::json!({"paidTier": {"name": "Google AI Ultra"}, "currentTier": {"id": "free"}})
            ),
            Some("Google AI Ultra".into())
        );
        assert_eq!(
            subscription_tier(&serde_json::json!({"currentTier": {"name": "Free"}, "ineligibleTiers": [{}]})),
            Some("Free (Restricted)".into())
        );
    }

    #[test]
    fn renews_then_reads_quota_and_skips_wrong_secret() {
        let (url, h) = serve(vec![
            ("/token", 401, r#"{"error":"invalid_client"}"#),
            ("/token", 200, r#"{"access_token":"ya29.ok","expires_in":3599}"#),
            (
                "/v1internal:loadCodeAssist",
                200,
                r#"{"cloudaicompanionProject":"proj-1","paidTier":{"name":"Google AI Pro"}}"#,
            ),
            ("/v1internal:retrieveUserQuotaSummary", 403, "{}"),
            ("/v1internal:retrieveUserQuotaSummary", 200, SUMMARY),
        ]);
        let cloud = Cloud {
            token_url: format!("{url}/token"),
            bases: vec![url.clone()],
            secrets: vec!["GOCSPX-bad".into(), "GOCSPX-good".into()],
        };
        let now = chrono::DateTime::parse_from_rfc3339("2029-12-31T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        let q = fetch(&cloud, "1//0renew-test-token-aaaaaaaaaaaaaaaa", now).unwrap();
        assert_eq!(q.tier.as_deref(), Some("Google AI Pro"));
        assert_eq!(q.readings.len(), 3);
        let five = q.readings.iter().find(|r| r.usage_type == "quota:gemini-5h").unwrap();
        assert_eq!(five.used_percent, Some(25.0));
        assert_eq!(five.window_minutes, Some(300));
        let third = q.readings.iter().find(|r| r.usage_type == "quota:3p-weekly").unwrap();
        assert_eq!(third.used_percent, Some(90.0));
        let seen = h.join().unwrap();
        assert!(seen[0].contains("client_secret=GOCSPX-bad"));
        assert!(seen[1].contains("client_secret=GOCSPX-good"));
        assert!(seen[2].contains("ANTIGRAVITY"));
        assert!(seen[3].contains("proj-1") && seen[3].contains("antigravity/hub/"));
        assert!(seen[4].contains("| antigravity |") && seen[4].contains("proj-1"));
    }

    #[test]
    fn expired_sign_in_is_a_credential_error() {
        let (url, h) = serve(vec![("/token", 400, r#"{"error":"invalid_grant"}"#)]);
        let cloud =
            Cloud { token_url: format!("{url}/token"), bases: vec![url.clone()], secrets: vec!["GOCSPX-good".into()] };
        let e = fetch(&cloud, "1//0expired-token-bbbbbbbbbbbbbbbbbb", chrono::Utc::now()).unwrap_err();
        assert!(matches!(e, Error::Credential(_)));
        h.join().unwrap();
    }
}
