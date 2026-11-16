use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ProviderId {
    #[serde(rename = "claude-code")]
    Claude,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "antigravity")]
    Antigravity,
}

impl ProviderId {
    pub const ALL: [ProviderId; 3] = [ProviderId::Claude, ProviderId::Codex, ProviderId::Antigravity];

    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::Claude => "claude-code",
            ProviderId::Codex => "codex",
            ProviderId::Antigravity => "antigravity",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ProviderId::Claude => "Claude Code",
            ProviderId::Codex => "Codex",
            ProviderId::Antigravity => "Antigravity",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == s)
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub key: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub organization: Option<String>,
    pub plan: Option<String>,
    pub auth_method: Option<String>,
}

impl Identity {
    pub fn short(&self) -> String {
        self.email.clone().or_else(|| self.display_name.clone()).unwrap_or_else(|| self.key.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Compatibility {
    Supported,
    Experimental,
    Untested,
    Unsupported,
    NotInstalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SwitchMode {
    Immediate,
    NewSessions,
    RestartRequired,
    Manual,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Support {
    Yes,
    No,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub multiple_accounts: bool,
    pub account_switching: bool,
    pub live_switching: bool,
    pub isolated_login: bool,
    pub login_replaces_current: bool,
    pub usage_monitoring: Support,
    pub restart_required: Support,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<PathBuf>,
    pub kind: ProcessKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessKind {
    Cli,
    App,
    Ide,
    Helper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub binary_path: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub compatibility: Compatibility,
    pub compatibility_note: Option<String>,
    pub experimental: bool,
    pub capabilities: Capabilities,
    pub processes: Vec<ProcessInfo>,
    pub switch_mode: SwitchMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthHealth {
    Ok,
    Expiring,
    Expired,
    NeedsReauth,
    Unknown,
}

impl AuthHealth {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthHealth::Ok => "ok",
            AuthHealth::Expiring => "expiring",
            AuthHealth::Expired => "expired",
            AuthHealth::NeedsReauth => "needs-reauth",
            AuthHealth::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "ok" => AuthHealth::Ok,
            "expiring" => AuthHealth::Expiring,
            "expired" => AuthHealth::Expired,
            "needs-reauth" => AuthHealth::NeedsReauth,
            _ => AuthHealth::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReading {
    pub usage_type: String,
    pub label: String,
    pub used_percent: Option<f64>,
    pub used_value: Option<f64>,
    pub limit_value: Option<f64>,
    pub window_minutes: Option<i64>,
    pub reset_at: Option<String>,
    pub source: String,
    pub confidence: String,
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_key: Option<String>,
}
