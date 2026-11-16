use altflip_core::UsageReading;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FIVE: f64 = 5.0 * 3600.0;
const WEEK: f64 = 7.0 * 86400.0;

pub fn desktop_dir(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Claude")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData/Roaming")).join("Claude")
    } else {
        home.join(".config/Claude")
    }
}

fn iso(secs: f64) -> String {
    chrono::DateTime::from_timestamp(secs as i64, 0)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

fn read_json(p: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&std::fs::read(p).ok()?).ok()
}

fn dirs(p: &Path) -> Vec<String> {
    std::fs::read_dir(p)
        .map(|rd| {
            rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()
        })
        .unwrap_or_default()
}

pub struct DesktopSessions {
    pub owners: HashMap<String, String>,
    pub org_accounts: HashMap<String, Vec<String>>,
}

pub fn desktop_sessions(desktop: &Path) -> DesktopSessions {
    let root = desktop.join("claude-code-sessions");
    let mut owners = HashMap::new();
    let mut org_accounts: HashMap<String, Vec<String>> = HashMap::new();
    for acc in dirs(&root) {
        for org in dirs(&root.join(&acc)) {
            org_accounts.entry(org.clone()).or_default().push(acc.clone());
            let Ok(rd) = std::fs::read_dir(root.join(&acc).join(&org)) else { continue };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if !(name.starts_with("local_") && name.ends_with(".json")) {
                    continue;
                }
                if let Some(sid) =
                    read_json(&e.path()).and_then(|j| j.get("cliSessionId")?.as_str().map(str::to_string))
                {
                    owners.insert(sid, acc.clone());
                }
            }
        }
    }
    DesktopSessions { owners, org_accounts }
}

#[derive(Clone, Copy)]
struct Sample {
    t: f64,
    fh: f64,
    sd: f64,
}

fn weekly_reset(raw: &[serde_json::Value], org: &str, now: f64) -> Option<f64> {
    let mut prev: Option<(f64, f64)> = None;
    let mut ranges: Vec<(f64, f64)> = Vec::new();
    for s in raw.iter().filter(|s| s.get("org").and_then(|o| o.as_str()) == Some(org)) {
        let (Some(t), Some(sd)) = (s.get("t").and_then(|t| t.as_f64()), s.pointer("/u/sd").and_then(|v| v.as_f64()))
        else {
            continue;
        };
        let ts = t / 1000.0;
        if let Some((pt, psd)) = prev {
            if sd <= 10.0 && psd - sd >= 3.0 && ts - pt < 2.0 * 86400.0 {
                ranges.push((pt, ts));
            }
        }
        prev = Some((ts, sd));
    }
    let latest = *ranges.last()?;
    let (mut lo, mut hi) = latest;
    for r in &ranges[..ranges.len() - 1] {
        let k = ((latest.0 - r.0) / WEEK).round();
        let (a, b) = (r.0 + k * WEEK, r.1 + k * WEEK);
        if lo.max(a) < hi.min(b) {
            lo = lo.max(a);
            hi = hi.min(b);
        }
    }
    let mut reset = (lo + hi) / 2.0;
    while reset <= now {
        reset += WEEK;
    }
    Some(reset)
}

fn reading(kind: &str, label: &str, mins: i64, pct: f64, reset: Option<f64>, observed: f64, key: &str) -> UsageReading {
    UsageReading {
        usage_type: kind.into(),
        label: label.into(),
        used_percent: Some(pct.clamp(0.0, 100.0)),
        used_value: None,
        limit_value: None,
        window_minutes: Some(mins),
        reset_at: reset.map(iso),
        source: "Claude Desktop history".into(),
        confidence: "estimated".into(),
        observed_at: iso(observed),
        account_key: Some(key.to_string()),
    }
}

pub fn plan_history(desktop: &Path, sessions: &DesktopSessions, now: f64) -> Vec<UsageReading> {
    let Some(j) = read_json(&desktop.join("plan-usage-history.json")) else { return Vec::new() };
    let Some(raw) = j.get("samples").and_then(|s| s.as_array()) else { return Vec::new() };
    let mut by_org: HashMap<String, Vec<Sample>> = HashMap::new();
    for s in raw {
        let (Some(org), Some(t)) = (s.get("org").and_then(|o| o.as_str()), s.get("t").and_then(|t| t.as_f64())) else {
            continue;
        };
        let Some(u) = s.get("u") else { continue };
        by_org.entry(org.to_string()).or_default().push(Sample {
            t: t / 1000.0,
            fh: u.get("fh").and_then(|v| v.as_f64()).unwrap_or(0.0),
            sd: u.get("sd").and_then(|v| v.as_f64()).unwrap_or(0.0),
        });
    }
    let mut out = Vec::new();
    for (org, mut samples) in by_org {
        samples.sort_by(|a, b| a.t.total_cmp(&b.t));
        let mut start: Option<f64> = None;
        let mut prev: Option<Sample> = None;
        let mut last: Option<Sample> = None;
        for s in samples {
            if let (Some(p), Some(st)) = (prev, start) {
                if s.fh <= 0.0 && p.fh > 0.0 && s.t < st + FIVE && s.sd >= p.sd {
                    continue;
                }
            }
            if let Some(p) = prev {
                if s.fh <= 0.0 && p.fh > 0.0 && s.t < p.t + FIVE && s.sd > p.sd + 0.5 {
                    continue;
                }
            }
            if s.fh <= 0.0 {
                start = None;
            } else if start.is_none_or(|st| s.t >= st + FIVE) || prev.is_some_and(|p| s.fh < p.fh) {
                start = Some(s.t);
            }
            prev = Some(s);
            last = Some(s);
        }
        let Some(last) = last else { continue };
        if now - last.t >= WEEK {
            continue;
        }
        let key = match sessions.org_accounts.get(&org).map(|v| v.as_slice()) {
            Some([one]) => format!("claude:{one}:{org}"),
            _ => format!("claude-org:{org}"),
        };
        let weekly = weekly_reset(raw, &org, now);
        match start.filter(|st| last.fh > 0.0 && st + FIVE > now) {
            Some(st) => out.push(reading("five_hour", "5-hour usage", 300, last.fh, Some(st + FIVE), last.t, &key)),
            None => out.push(reading("five_hour", "5-hour usage", 300, 0.0, None, last.t, &key)),
        }
        out.push(reading("seven_day", "Weekly usage", 10080, last.sd, weekly, last.t, &key));
    }
    out
}

fn status_files(config_dir: &Path) -> Vec<(String, PathBuf, std::time::SystemTime)> {
    let mut out = Vec::new();
    for sub in [".altflip-status", ".accountdeck-status"] {
        let Ok(rd) = std::fs::read_dir(config_dir.join(sub)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(sid) = name.strip_suffix(".json") else { continue };
            let Ok(m) = e.metadata().and_then(|m| m.modified()) else { continue };
            out.push((sid.to_string(), e.path(), m));
        }
    }
    out.sort_by_key(|f| std::cmp::Reverse(f.2));
    out
}

pub fn parse_status(v: &serde_json::Value, observed: &str, now: f64) -> Vec<UsageReading> {
    let Some(rl) = v.get("rate_limits").filter(|r| r.as_object().is_some_and(|o| !o.is_empty())) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, label, mins) in [("five_hour", "5-hour usage", 300), ("seven_day", "Weekly usage", 10080)] {
        let Some(w) = rl.get(key) else { continue };
        let Some(pct) = w.get("used_percentage").and_then(|p| p.as_f64()) else { continue };
        let reset = w.get("resets_at").and_then(|r| r.as_f64());
        let (pct, reset) = if reset.is_some_and(|r| r < now) { (0.0, None) } else { (pct, reset) };
        out.push(UsageReading {
            usage_type: key.into(),
            label: label.into(),
            used_percent: Some(pct.clamp(0.0, 100.0)),
            used_value: None,
            limit_value: None,
            window_minutes: Some(mins),
            reset_at: reset.map(iso),
            source: "Claude Code status line".into(),
            confidence: "provider-reported".into(),
            observed_at: observed.to_string(),
            account_key: None,
        });
    }
    out
}

pub fn status_line(config_dir: &Path, sessions: &DesktopSessions, now: f64) -> Vec<UsageReading> {
    for (sid, path, mtime) in status_files(config_dir) {
        if sessions.owners.contains_key(&sid) {
            continue;
        }
        let Some(v) = read_json(&path) else { continue };
        let observed = chrono::DateTime::<chrono::Utc>::from(mtime).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let r = parse_status(&v, &observed, now);
        if !r.is_empty() {
            return r;
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> f64 {
        chrono::Utc::now().timestamp() as f64
    }

    #[test]
    fn plan_history_estimates_windows_and_maps_org_to_account() {
        let dir = tempfile::tempdir().unwrap();
        let n = now();
        let ms = |s: f64| s * 1000.0;
        let samples = serde_json::json!({"version": 1, "samples": [
            {"t": ms(n - 8.0 * 86400.0), "org": "o1", "u": {"fh": 5, "sd": 60}},
            {"t": ms(n - 8.0 * 86400.0 + 3600.0), "org": "o1", "u": {"fh": 0, "sd": 2}},
            {"t": ms(n - 3600.0), "org": "o1", "u": {"fh": 10, "sd": 30}},
            {"t": ms(n - 600.0), "org": "o1", "u": {"fh": 25, "sd": 33}},
            {"t": ms(n - 600.0), "org": "o2", "u": {"fh": 0, "sd": 12}}
        ]});
        std::fs::write(dir.path().join("plan-usage-history.json"), samples.to_string()).unwrap();
        let sess_dir = dir.path().join("claude-code-sessions/acc-1/o1");
        std::fs::create_dir_all(&sess_dir).unwrap();
        std::fs::write(sess_dir.join("local_a.json"), r#"{"cliSessionId":"s-desk"}"#).unwrap();
        let sessions = desktop_sessions(dir.path());
        assert_eq!(sessions.owners.get("s-desk").map(String::as_str), Some("acc-1"));
        let r = plan_history(dir.path(), &sessions, n);
        let o1: Vec<_> = r.iter().filter(|u| u.account_key.as_deref() == Some("claude:acc-1:o1")).collect();
        assert_eq!(o1.len(), 2);
        let five = o1.iter().find(|u| u.usage_type == "five_hour").unwrap();
        assert_eq!(five.used_percent, Some(25.0));
        assert!(five.reset_at.is_some());
        assert_eq!(five.confidence, "estimated");
        let week = o1.iter().find(|u| u.usage_type == "seven_day").unwrap();
        assert_eq!(week.used_percent, Some(33.0));
        assert!(week.reset_at.is_some());
        assert!(r.iter().any(|u| u.account_key.as_deref() == Some("claude-org:o2") && u.usage_type == "seven_day"));
    }

    #[test]
    fn status_line_skips_desktop_sessions_and_reset_windows() {
        let dir = tempfile::tempdir().unwrap();
        let sd = dir.path().join(".accountdeck-status");
        std::fs::create_dir_all(&sd).unwrap();
        let n = now();
        std::fs::write(
            sd.join("cli.json"),
            serde_json::json!({"session_id": "cli", "rate_limits": {"five_hour": {"used_percentage": 40, "resets_at": n + 600.0}, "seven_day": {"used_percentage": 70, "resets_at": n - 10.0}}}).to_string(),
        )
        .unwrap();
        let mut owners = DesktopSessions { owners: HashMap::new(), org_accounts: HashMap::new() };
        let r = status_line(dir.path(), &owners, n);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].used_percent, Some(40.0));
        assert_eq!(r[1].used_percent, Some(0.0));
        owners.owners.insert("cli".into(), "acc".into());
        assert!(status_line(dir.path(), &owners, n).is_empty());
    }
}
