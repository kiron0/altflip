use ts_rs::TS;
use altflip_core::{fsutil, Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const SCRIPT_NAME: &str = "altflip-status.sh";
pub const STATUS_DIR: &str = ".altflip-status";
const PREVIOUS: &str = "previous-command";
const LEGACY_MARKERS: [&str; 2] = ["accountdeck/claude-status.sh", "altflip-status.sh"];

const SCRIPT: &str = r#"#!/bin/sh
d="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
o="$d/.altflip-status"
input=$(cat)
sid=$(printf '%s' "$input" | sed -n 's/.*"session_id" *: *"\([A-Za-z0-9_-]*\)".*/\1/p' | head -n 1)
if [ -n "$sid" ]; then
  mkdir -p "$o"
  printf '%s' "$input" > "$o/.$sid.tmp" && mv -f "$o/.$sid.tmp" "$o/$sid.json"
  find "$o" -name '*.json' -mtime +8 -delete 2>/dev/null
fi
if [ -s "$o/previous-command" ]; then
  printf '%s' "$input" | sh -c "$(cat "$o/previous-command")"
fi
"#;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct StatusLineState {
    pub installed: bool,
    pub script_path: String,
    pub chained_command: Option<String>,
    pub last_reading_at: Option<String>,
    pub summary: String,
}

fn script_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SCRIPT_NAME)
}

fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

fn previous_path(config_dir: &Path) -> PathBuf {
    config_dir.join(STATUS_DIR).join(PREVIOUS)
}

fn our_command(config_dir: &Path) -> String {
    format!("sh \"{}\"", script_path(config_dir).display())
}

fn read_settings(config_dir: &Path) -> Result<serde_json::Value> {
    match fsutil::read_optional(&settings_path(config_dir))? {
        None => Ok(serde_json::json!({})),
        Some(b) if b.iter().all(|c| c.is_ascii_whitespace()) => Ok(serde_json::json!({})),
        Some(b) => {
            let v: serde_json::Value = serde_json::from_slice(&b).map_err(|_| {
                Error::Invalid("Claude Code's settings.json is not valid JSON, so AltFlip left it unchanged".into())
            })?;
            if !v.is_object() {
                return Err(Error::Invalid("Claude Code's settings.json is not a JSON object".into()));
            }
            Ok(v)
        }
    }
}

fn current_command(settings: &serde_json::Value) -> Option<String> {
    settings
        .pointer("/statusLine/command")
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .filter(|c| !c.trim().is_empty())
}

fn is_ours_or_legacy(cmd: &str) -> bool {
    LEGACY_MARKERS.iter().any(|m| cmd.contains(m))
}

fn last_reading(config_dir: &Path) -> Option<String> {
    let rd = std::fs::read_dir(config_dir.join(STATUS_DIR)).ok()?;
    let newest = rd
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .filter_map(|e| e.metadata().and_then(|m| m.modified()).ok())
        .max()?;
    Some(chrono::DateTime::<chrono::Utc>::from(newest).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

pub fn state(config_dir: &Path) -> StatusLineState {
    let settings = read_settings(config_dir).unwrap_or_else(|_| serde_json::json!({}));
    let installed =
        current_command(&settings).is_some_and(|c| c.contains(SCRIPT_NAME)) && script_path(config_dir).is_file();
    let chained = std::fs::read_to_string(previous_path(config_dir)).ok().filter(|c| !c.trim().is_empty());
    let mut st = StatusLineState {
        installed,
        script_path: script_path(config_dir).display().to_string(),
        chained_command: if installed { chained } else { None },
        summary: String::new(),
        last_reading_at: last_reading(config_dir),
    };
    st.summary = summary(&st);
    st
}

fn summary(st: &StatusLineState) -> String {
    if !st.installed {
        return "Claude Code reports usage to its status line after every reply. AltFlip adds a small script there so the Usage page updates instantly and never waits on Anthropic.".into();
    }
    let mut out = match &st.last_reading_at {
        Some(t) => format!("Last reading {}.", altflip_core::text::relative_time(Some(t), chrono::Utc::now())),
        None => "Waiting for the next Claude Code reply.".into(),
    };
    if let Some(c) = &st.chained_command {
        out.push_str(&format!(" Your previous status line still runs: {c}"));
    }
    out
}

pub fn install(config_dir: &Path) -> Result<StatusLineState> {
    let mut settings = read_settings(config_dir)?;
    let current = current_command(&settings);
    fsutil::atomic_write(&script_path(config_dir), SCRIPT.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(script_path(config_dir), std::fs::Permissions::from_mode(0o755));
    }
    match current.as_deref() {
        Some(c) if !is_ours_or_legacy(c) => fsutil::atomic_write(&previous_path(config_dir), c.as_bytes())?,
        Some(c) if c.contains("accountdeck/claude-status.sh") => fsutil::remove_if_exists(&previous_path(config_dir))?,
        _ => {}
    }
    let obj = settings.as_object_mut().ok_or_else(|| Error::Invalid("settings.json is not a JSON object".into()))?;
    let line = obj.entry("statusLine").or_insert_with(|| serde_json::json!({}));
    if !line.is_object() {
        *line = serde_json::json!({});
    }
    line["type"] = "command".into();
    line["command"] = our_command(config_dir).into();
    let mut bytes = serde_json::to_vec_pretty(&settings)?;
    bytes.push(b'\n');
    fsutil::atomic_write(&settings_path(config_dir), &bytes)?;
    Ok(state(config_dir))
}

pub fn uninstall(config_dir: &Path) -> Result<StatusLineState> {
    let mut settings = read_settings(config_dir)?;
    if current_command(&settings).is_some_and(|c| c.contains(SCRIPT_NAME)) {
        let previous = std::fs::read_to_string(previous_path(config_dir)).ok().filter(|c| !c.trim().is_empty());
        let obj =
            settings.as_object_mut().ok_or_else(|| Error::Invalid("settings.json is not a JSON object".into()))?;
        match previous {
            Some(p) => {
                if let Some(line) = obj.get_mut("statusLine").filter(|l| l.is_object()) {
                    line["command"] = p.into();
                }
            }
            None => {
                obj.remove("statusLine");
            }
        }
        let mut bytes = serde_json::to_vec_pretty(&settings)?;
        bytes.push(b'\n');
        fsutil::atomic_write(&settings_path(config_dir), &bytes)?;
    }
    fsutil::remove_if_exists(&previous_path(config_dir))?;
    fsutil::remove_if_exists(&script_path(config_dir))?;
    Ok(state(config_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(dir: &Path) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(settings_path(dir)).unwrap()).unwrap()
    }

    #[test]
    fn installs_into_empty_config_and_removes_cleanly() {
        let d = tempfile::tempdir().unwrap();
        let s = install(d.path()).unwrap();
        assert!(s.installed);
        assert!(script_path(d.path()).is_file());
        assert!(read(d.path())["statusLine"]["command"].as_str().unwrap().contains(SCRIPT_NAME));
        let s = uninstall(d.path()).unwrap();
        assert!(!s.installed);
        assert!(read(d.path()).get("statusLine").is_none());
        assert!(!script_path(d.path()).exists());
    }

    #[test]
    fn chains_foreign_command_and_restores_it() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            settings_path(d.path()),
            r#"{"model":"opus","statusLine":{"type":"command","command":"my-line --fancy","padding":1}}"#,
        )
        .unwrap();
        let s = install(d.path()).unwrap();
        assert_eq!(s.chained_command.as_deref(), Some("my-line --fancy"));
        let v = read(d.path());
        assert_eq!(v["model"], "opus");
        assert_eq!(v["statusLine"]["padding"], 1);
        install(d.path()).unwrap();
        assert_eq!(state(d.path()).chained_command.as_deref(), Some("my-line --fancy"));
        uninstall(d.path()).unwrap();
        assert_eq!(read(d.path())["statusLine"]["command"], "my-line --fancy");
    }

    #[test]
    fn replaces_legacy_accountdeck_script_without_chaining() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            settings_path(d.path()),
            r#"{"statusLine":{"type":"command","command":"sh \"$HOME/.accountdeck/claude-status.sh\""}}"#,
        )
        .unwrap();
        let s = install(d.path()).unwrap();
        assert!(s.installed);
        assert_eq!(s.chained_command, None);
    }

    #[test]
    fn refuses_to_touch_invalid_settings() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(settings_path(d.path()), "{ not json").unwrap();
        assert!(install(d.path()).is_err());
        assert_eq!(std::fs::read_to_string(settings_path(d.path())).unwrap(), "{ not json");
    }

    #[cfg(unix)]
    #[test]
    fn script_saves_status_and_runs_chained_command() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(settings_path(d.path()), r#"{"statusLine":{"type":"command","command":"cat"}}"#).unwrap();
        install(d.path()).unwrap();
        let mut child = std::process::Command::new("sh")
            .arg(script_path(d.path()))
            .env("CLAUDE_CONFIG_DIR", d.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        let input = r#"{"session_id":"abc-1","rate_limits":{}}"#;
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout), input);
        assert_eq!(std::fs::read_to_string(d.path().join(STATUS_DIR).join("abc-1.json")).unwrap(), input);
        assert!(state(d.path()).last_reading_at.is_some());
    }
}
