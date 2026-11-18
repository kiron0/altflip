use altflip_core::{Error, Result};
use altflip_database::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub onboarded: bool,
    #[ts(type = "\"system\" | \"light\" | \"dark\"")]
    pub theme: String,
    pub launch_at_login: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub show_dock_icon: bool,
    #[ts(type = "\"popover\" | \"dashboard\"")]
    pub tray_click: String,
    pub global_shortcut: Option<String>,
    pub notifications: bool,
    pub notify_switch: bool,
    pub notify_usage: bool,
    pub notify_auth: bool,
    pub usage_monitoring: bool,
    pub usage_refresh_minutes: u64,
    pub usage_threshold: u64,
    pub show_reset_countdowns: bool,
    pub warn_running_sessions: bool,
    pub live_usage: bool,
    pub diagnostic_level: String,
}

pub fn default_settings() -> Value {
    json!({
        "onboarded": false,
        "theme": "system",
        "launchAtLogin": false,
        "startMinimized": true,
        "closeToTray": true,
        "showDockIcon": false,
        "trayClick": "popover",
        "globalShortcut": null,
        "notifications": true,
        "notifySwitch": true,
        "notifyUsage": true,
        "notifyAuth": true,
        "usageMonitoring": true,
        "usageRefreshMinutes": 15,
        "usageThreshold": 80,
        "showResetCountdowns": true,
        "warnRunningSessions": true,
        "liveUsage": true,
        "diagnosticLevel": "info"
    })
}

pub fn load(db: &Store) -> Result<Settings> {
    let mut merged: Map<String, Value> = default_settings().as_object().cloned().unwrap_or_default();
    for (k, v) in db.settings()? {
        if validate(&k, &v).is_ok() {
            merged.insert(k, v);
        }
    }
    Ok(serde_json::from_value(Value::Object(merged))?)
}

pub fn validate(key: &str, value: &Value) -> Result<()> {
    let defaults = default_settings();
    let Some(def) = defaults.get(key) else { return Err(Error::Invalid(format!("Unknown setting {key}"))) };
    let ok = match key {
        "theme" => matches!(value.as_str(), Some("system" | "light" | "dark")),
        "trayClick" => matches!(value.as_str(), Some("popover" | "dashboard")),
        "diagnosticLevel" => matches!(value.as_str(), Some("error" | "warn" | "info" | "debug")),
        "globalShortcut" => value.is_null() || value.as_str().is_some_and(|s| s.len() < 64),
        "usageRefreshMinutes" => value.as_u64().is_some_and(|v| (5..=240).contains(&v)),
        "usageThreshold" => value.as_u64().is_some_and(|v| (10..=100).contains(&v)),
        _ => std::mem::discriminant(def) == std::mem::discriminant(value),
    };
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!("Invalid value for {key}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_load_and_validate() {
        let db = Store::open_in_memory().unwrap();
        load(&db).unwrap();
        assert!(validate("theme", &json!("dark")).is_ok());
        assert!(validate("theme", &json!("neon")).is_err());
        assert!(validate("notifications", &json!("yes")).is_err());
        assert!(validate("nope", &json!(1)).is_err());
        assert!(validate("usageRefreshMinutes", &json!(1)).is_err());
        db.set_setting("theme", &json!(42)).unwrap();
        assert_eq!(load(&db).unwrap().theme, "system");
    }
}
