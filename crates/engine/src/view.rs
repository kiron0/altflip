use crate::{sign_in_hint, AccountView, LiveAccount};
use altflip_core::text::{self, Badge, Tone};
use altflip_core::{AuthHealth, Compatibility, ProcessKind, ProviderId, ProviderInfo, Support, SwitchMode};
use altflip_database::{AccountRow, ActiveRow, ActivityRow, SwitchRecord, UsageRow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

const STALE_MINUTES: i64 = 15;
const DEFAULT_RANGE_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "bindings.ts")]
pub enum RowState {
    Active,
    NewSessions,
    Drifted,
    SignedOut,
    External,
    Expired,
    Error,
    Idle,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct UsageView {
    #[serde(flatten)]
    pub row: UsageRow,
    pub percent: Option<u32>,
    pub reset_passed: bool,
    pub reset_text: Option<String>,
    pub left_text: String,
    pub countdown: Option<String>,
    pub observed_text: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct InlineUsage {
    pub text: String,
    pub tone: Tone,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct Freshness {
    pub text: String,
    pub stale: bool,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "bindings.ts")]
pub enum LiveKind {
    Error,
    SignedOut,
    Unsaved,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct LiveLine {
    pub kind: LiveKind,
    pub lead: String,
    pub who: Option<String>,
    pub tail: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct ProviderText {
    pub compat_badge: Badge,
    pub mode_label: String,
    pub subtitle: String,
    pub sign_in_hint: String,
    pub usage_supported: bool,
    pub usage_note: String,
    pub live_line: Option<LiveLine>,
    pub drift_note: Option<String>,
    pub featured_account_id: Option<String>,
    pub see_all: bool,
    pub more_label: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct OverviewStats {
    pub accounts: usize,
    pub tools: usize,
    pub tools_text: String,
    pub active_selections: usize,
    pub active_percent: u32,
    pub attention: usize,
    pub needs_review: bool,
    pub usage_window_rolled: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct SwitchView {
    #[serde(flatten)]
    pub record: SwitchRecord,
    pub status_badge: Badge,
    pub mode_label: String,
    pub duration_text: Option<String>,
    pub took_text: String,
    pub started_text: String,
    pub started_full: String,
    pub from_text: String,
    pub to_text: String,
    pub undoable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "bindings.ts")]
pub enum Category {
    Switch,
    Account,
    Problem,
    System,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts", optional_fields)]
pub struct FeedQuery {
    pub query: Option<String>,
    pub provider: Option<ProviderId>,
    pub category: Option<Category>,
    pub range_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct FeedItem {
    pub key: String,
    pub at: String,
    pub clock: String,
    pub full_time: String,
    pub provider: Option<ProviderId>,
    pub provider_name: String,
    pub category: Category,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub account_name: Option<String>,
    pub switch: Option<SwitchView>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct FeedGroup {
    pub day: String,
    pub items: Vec<FeedItem>,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct FeedCounts {
    pub all: usize,
    pub switch: usize,
    pub account: usize,
    pub problem: usize,
    pub system: usize,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct FeedStat {
    pub label: String,
    pub value: String,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct ProviderOption {
    pub id: ProviderId,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct Feed {
    pub groups: Vec<FeedGroup>,
    pub counts: FeedCounts,
    pub stats: Vec<FeedStat>,
    pub providers: Vec<ProviderOption>,
    pub total: usize,
    pub filtering: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct Notice {
    pub text: String,
    pub detail: String,
}

pub fn row_state(active: Option<&ActiveRow>, row: &AccountRow) -> RowState {
    if let Some(a) = active {
        return match a.activation_state.as_str() {
            "new-sessions" => RowState::NewSessions,
            "drifted" => RowState::Drifted,
            "signed-out" => RowState::SignedOut,
            "external" => RowState::External,
            _ => RowState::Active,
        };
    }
    if matches!(row.health, AuthHealth::Expired | AuthHealth::NeedsReauth) {
        return RowState::Expired;
    }
    if row.last_error.is_some() {
        return RowState::Error;
    }
    RowState::Idle
}

pub fn state_badge(s: RowState) -> Option<Badge> {
    match s {
        RowState::Drifted => Some(Badge::new("Changed outside", Tone::Warn)),
        RowState::SignedOut => Some(Badge::new("Signed out", Tone::Warn)),
        RowState::Expired => Some(Badge::new("Sign-in expired", Tone::Danger)),
        RowState::Error => Some(Badge::new("Last switch failed", Tone::Danger)),
        _ => None,
    }
}

pub fn health_badge(h: AuthHealth) -> Badge {
    let text = match h {
        AuthHealth::Ok => "Signed in",
        AuthHealth::Expiring => "Expiring soon",
        AuthHealth::Expired => "Sign-in expired",
        AuthHealth::NeedsReauth => "Needs sign-in",
        AuthHealth::Unknown => "Unknown",
    };
    let tone = match h {
        AuthHealth::Ok => Tone::Ok,
        AuthHealth::Unknown => Tone::Neutral,
        _ => Tone::Danger,
    };
    Badge::new(text, tone)
}

pub fn compat_badge(c: Compatibility) -> Badge {
    match c {
        Compatibility::Supported => Badge::new("Supported", Tone::Ok),
        Compatibility::Experimental => Badge::new("Experimental", Tone::Warn),
        Compatibility::Untested => Badge::new("Untested version", Tone::Warn),
        Compatibility::Unsupported => Badge::new("Switching unavailable", Tone::Danger),
        Compatibility::NotInstalled => Badge::new("Not installed", Tone::Neutral),
    }
}

pub fn mode_label(m: SwitchMode) -> &'static str {
    match m {
        SwitchMode::Immediate => "Instant",
        SwitchMode::NewSessions => "Applies to new sessions",
        SwitchMode::RestartRequired => "Restart required",
        SwitchMode::Manual => "Manual",
        SwitchMode::Unsupported => "Switching unavailable",
    }
}

pub fn mode_text(m: SwitchMode) -> &'static str {
    match m {
        SwitchMode::Immediate => "Takes effect immediately.",
        SwitchMode::NewSessions => {
            "New sessions use the selected account. Sessions already running keep their current account."
        }
        SwitchMode::RestartRequired => "The app must restart to use the selected account.",
        SwitchMode::Manual => "Requires a manual step in the provider app.",
        SwitchMode::Unsupported => "Switching is not available.",
    }
}

fn mode_label_str(s: &str) -> String {
    serde_json::from_value::<SwitchMode>(serde_json::Value::String(s.into()))
        .map(|m| mode_label(m).to_string())
        .unwrap_or_else(|_| s.to_string())
}

pub fn status_badge(status: &str) -> Badge {
    match status {
        "succeeded" => Badge::new("Verified", Tone::Ok),
        "rolled-back" => Badge::new("Rolled back", Tone::Warn),
        "needs-recovery" => Badge::new("Needs recovery", Tone::Danger),
        "cancelled" => Badge::new("Cancelled", Tone::Neutral),
        "abandoned" => Badge::new("Interrupted", Tone::Neutral),
        "recovered" => Badge::new("Restored", Tone::Info),
        "resolved" => Badge::new("Resolved", Tone::Neutral),
        "started" => Badge::new("In progress", Tone::Info),
        "applying" => Badge::new("Interrupted", Tone::Warn),
        other => Badge::new(other, Tone::Neutral),
    }
}

fn reset_passed(u: &UsageRow, now: DateTime<Utc>) -> bool {
    u.reading.reset_at.as_deref().and_then(text::parse).is_some_and(|t| t <= now)
}

pub fn usage_view(row: UsageRow, now: DateTime<Utc>) -> UsageView {
    let passed = reset_passed(&row, now);
    let percent = row.reading.used_percent.map(|p| if passed { 0 } else { p.round().clamp(0.0, 999.0) as u32 });
    let left_text = if passed {
        "New window started".into()
    } else {
        format!("{}% left", 100u32.saturating_sub(percent.unwrap_or(0)))
    };
    UsageView {
        percent,
        reset_passed: passed,
        reset_text: if passed { None } else { text::reset_text(row.reading.reset_at.as_deref(), now) },
        left_text,
        countdown: text::countdown(row.reading.reset_at.as_deref(), now),
        observed_text: text::relative_time(Some(&row.reading.observed_at), now),
        row,
    }
}

fn inline_usage(usage: &[UsageView], now: DateTime<Utc>) -> Option<InlineUsage> {
    let u = usage
        .iter()
        .filter(|u| {
            u.row.reading.used_percent.is_some()
                && !u.row.reading.reset_at.as_deref().and_then(text::parse).is_some_and(|t| t < now)
        })
        .max_by(|a, b| {
            a.row.reading.used_percent.unwrap_or(0.0).total_cmp(&b.row.reading.used_percent.unwrap_or(0.0))
        })?;
    let pct = u.row.reading.used_percent?.round() as i64;
    let tone = if pct >= 90 {
        Tone::Danger
    } else if pct >= 70 {
        Tone::Warn
    } else {
        Tone::Neutral
    };
    Some(InlineUsage {
        text: match &u.countdown {
            Some(c) => format!("{pct}% · {c}"),
            None => format!("{pct}%"),
        },
        tone,
        title: format!("{}: {pct}% used · {}", u.row.reading.label, u.row.reading.source),
    })
}

fn freshness(usage: &[UsageView], now: DateTime<Utc>) -> Option<Freshness> {
    let latest = usage.iter().max_by(|a, b| a.row.reading.observed_at.cmp(&b.row.reading.observed_at))?;
    let at = text::parse(&latest.row.reading.observed_at);
    Some(Freshness {
        text: format!("Updated {}", latest.observed_text),
        stale: at.is_none_or(|t| (now - t).num_minutes() > STALE_MINUTES),
        source: latest.row.reading.source.clone(),
    })
}

pub fn account_view(
    row: AccountRow,
    active: Option<&ActiveRow>,
    usage: Vec<UsageRow>,
    usage_error: Option<String>,
    now: DateTime<Utc>,
) -> AccountView {
    let state = row_state(active, &row);
    let usage: Vec<UsageView> = usage.into_iter().map(|u| usage_view(u, now)).collect();
    let plan_label = text::plan_label(row.subscription_type.as_deref());
    let subtitle = [row.email.clone().or_else(|| row.display_name.clone()), plan_label.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    AccountView {
        active: active.is_some(),
        activation_state: active.map(|a| a.activation_state.clone()),
        verified_at: active.and_then(|a| a.verified_at.clone()),
        row_state: state,
        state_badge: state_badge(state),
        plan_label,
        subtitle: if subtitle.is_empty() { "No details".into() } else { subtitle },
        health_badge: health_badge(row.health),
        needs_sign_in: matches!(row.health, AuthHealth::Expired | AuthHealth::NeedsReauth),
        inline_usage: inline_usage(&usage, now),
        freshness: freshness(&usage, now),
        added_text: text::relative_time(Some(&row.created_at), now),
        last_used_text: text::relative_time(row.last_used_at.as_deref(), now),
        captured_text: text::relative_time(row.last_captured_at.as_deref(), now),
        verified_text: active.and_then(|a| a.verified_at.as_deref()).map(|v| text::relative_time(Some(v), now)),
        usage,
        usage_error,
        row,
    }
}

fn usage_supported(info: &ProviderInfo) -> bool {
    info.capabilities.usage_monitoring != Support::No
}

fn usage_empty_text(id: ProviderId) -> &'static str {
    match id {
        ProviderId::Antigravity => {
            "No reading yet. AltFlip reads live quota for each saved account from Google every 2 minutes. Antigravity or agy must be installed."
        }
        _ => "No reading yet. Press Refresh.",
    }
}

fn live_line(info: &ProviderInfo, live: Option<&LiveAccount>, live_error: Option<&str>) -> Option<LiveLine> {
    if let Some(e) = live_error {
        return Some(LiveLine {
            kind: LiveKind::Error,
            lead: format!("Could not read the current sign-in: {e}"),
            who: None,
            tail: String::new(),
        });
    }
    let Some(live) = live else {
        return Some(LiveLine {
            kind: LiveKind::SignedOut,
            lead: "Not signed in right now.".into(),
            who: None,
            tail: String::new(),
        });
    };
    if live.saved_account_id.is_some() {
        return None;
    }
    let who = live
        .identity
        .email
        .clone()
        .or_else(|| live.identity.display_name.clone())
        .unwrap_or_else(|| "an unknown account".into());
    Some(LiveLine {
        kind: LiveKind::Unsaved,
        lead: format!("{} is signed in as", info.name),
        who: Some(who),
        tail: ", which isn't saved yet.".into(),
    })
}

pub fn provider_text(
    info: &ProviderInfo,
    accounts: &[AccountView],
    active_account_id: Option<&str>,
    activation_state: Option<&str>,
    live: Option<&LiveAccount>,
    live_error: Option<&str>,
) -> ProviderText {
    let cli = info.processes.iter().filter(|p| p.kind == ProcessKind::Cli).count();
    let apps = info.processes.iter().filter(|p| matches!(p.kind, ProcessKind::App | ProcessKind::Ide)).count();
    let mut parts =
        vec![info.version.clone().unwrap_or_else(|| "Version unknown".into()), mode_label(info.switch_mode).into()];
    let running: Vec<String> = [(cli, "running session"), (apps, "open app")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| text::plural(n, w))
        .collect();
    if !running.is_empty() {
        parts.push(running.join(", "));
    }
    let active = active_account_id.and_then(|id| accounts.iter().find(|a| a.row.id == id));
    let see_all = accounts.len() > 1;
    ProviderText {
        compat_badge: compat_badge(info.compatibility),
        mode_label: mode_label(info.switch_mode).into(),
        subtitle: parts.join(" · "),
        sign_in_hint: sign_in_hint(info.id).into(),
        usage_supported: usage_supported(info),
        usage_note: if usage_supported(info) {
            usage_empty_text(info.id).into()
        } else {
            format!(
                "{} doesn't report usage limits to other apps, so AltFlip can't show them. Switching accounts still works.",
                info.name
            )
        },
        live_line: live_line(info, live, live_error),
        drift_note: active
            .filter(|_| activation_state == Some("drifted"))
            .map(|a| format!("Selected: {}. The sign-in was changed outside AltFlip.", a.row.label)),
        featured_account_id: active.or(accounts.first()).map(|a| a.row.id.clone()),
        see_all,
        more_label: if see_all {
            format!("See all {} accounts", accounts.len())
        } else {
            format!("Add another {} account", info.name)
        },
    }
}

pub struct StatInput<'a> {
    pub installed: bool,
    pub activation_state: Option<&'a str>,
    pub has_active: bool,
    pub accounts: &'a [AccountView],
}

pub fn overview_stats(providers: &[StatInput], incomplete: usize, now: DateTime<Utc>) -> OverviewStats {
    let accounts = providers.iter().map(|p| p.accounts.len()).sum();
    let tools = providers.iter().filter(|p| p.installed).count();
    let active_selections = providers.iter().filter(|p| p.has_active).count();
    let attention = incomplete
        + providers.iter().filter(|p| matches!(p.activation_state, Some("drifted" | "signed-out"))).count()
        + providers.iter().flat_map(|p| p.accounts.iter()).filter(|a| a.needs_sign_in).count();
    let usage_window_rolled =
        providers.iter().flat_map(|p| p.accounts.iter()).flat_map(|a| a.usage.iter()).any(|u| {
            let r = &u.row.reading;
            match (r.reset_at.as_deref().and_then(text::parse), text::parse(&r.observed_at)) {
                (Some(reset), Some(seen)) => reset <= now && seen < reset,
                _ => false,
            }
        });
    OverviewStats {
        accounts,
        tools,
        tools_text: format!("{tools} of {}", providers.len()),
        active_selections,
        active_percent: if tools == 0 { 0 } else { ((active_selections * 100) as f64 / tools as f64).round() as u32 },
        attention,
        needs_review: incomplete > 0 || providers.iter().any(|p| p.activation_state == Some("drifted")),
        usage_window_rolled,
    }
}

pub fn switch_view(record: SwitchRecord, undoable: bool, now: DateTime<Utc>) -> SwitchView {
    let duration_text = text::elapsed_ms(&record.started_at, record.completed_at.as_deref()).map(text::duration_text);
    let took_text = match (&duration_text, &record.completed_at) {
        (Some(d), _) => d.clone(),
        (None, Some(_)) => "—".into(),
        (None, None) => "Did not finish".into(),
    };
    SwitchView {
        status_badge: status_badge(&record.status),
        mode_label: mode_label_str(&record.switch_mode),
        duration_text,
        took_text,
        started_text: text::relative_time(Some(&record.started_at), now),
        started_full: text::full_time(&record.started_at),
        from_text: record.from_label.clone().unwrap_or_else(|| "No account".into()),
        to_text: record.to_label.clone().unwrap_or_else(|| "—".into()),
        undoable,
        record,
    }
}

pub fn undoable_ids(history: &[SwitchRecord]) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut ids = HashSet::new();
    for r in history {
        if seen.contains(&r.provider) {
            continue;
        }
        if r.status == "succeeded" {
            ids.insert(r.id.clone());
        }
        if r.status == "succeeded" || r.status == "recovered" {
            seen.insert(r.provider);
        }
    }
    ids
}

fn event_kind(kind: &str) -> Option<(&'static str, Category)> {
    Some(match kind {
        "added" => ("Account added", Category::Account),
        "updated" => ("Account updated", Category::Account),
        "renamed" => ("Account renamed", Category::Account),
        "removed" => ("Account removed", Category::Account),
        "renewed" => ("Sign-in renewed", Category::Account),
        "external" => ("Switched outside AltFlip", Category::Switch),
        "drift" => ("Unsaved sign-in detected", Category::Problem),
        "interrupted" => ("Switch interrupted", Category::Problem),
        "needs-recovery" => ("Needs recovery", Category::Problem),
        "usage" => ("Usage unavailable", Category::Problem),
        "recovered" => ("Previous sign-in restored", Category::System),
        "settings" => ("Settings changed", Category::System),
        _ => return None,
    })
}

fn provider_name(p: Option<ProviderId>) -> String {
    p.map(|p| p.name().to_string()).unwrap_or_else(|| "AltFlip".into())
}

pub fn feed(
    history: Vec<SwitchRecord>,
    log: Vec<ActivityRow>,
    account_label: impl Fn(&str) -> Option<String>,
    q: &FeedQuery,
    now: DateTime<Utc>,
) -> Feed {
    let undoable = undoable_ids(&history);
    let mut items: Vec<FeedItem> = history
        .into_iter()
        .map(|r| {
            let ok = r.status == "succeeded" || r.status == "recovered";
            let text = match (&r.message, r.status.as_str()) {
                (Some(m), _) => m.clone(),
                (None, "succeeded") => "Switched and verified".into(),
                (None, s) => s.to_string(),
            };
            let undo = undoable.contains(&r.id);
            let view = switch_view(r, undo, now);
            FeedItem {
                key: format!("s-{}", view.record.id),
                at: view.record.started_at.clone(),
                clock: text::clock(&view.record.started_at),
                full_time: view.started_full.clone(),
                provider: Some(view.record.provider),
                provider_name: provider_name(Some(view.record.provider)),
                category: if ok { Category::Switch } else { Category::Problem },
                kind: view.record.status.clone(),
                title: format!("{} → {}", view.from_text, view.to_text),
                text,
                account_name: None,
                switch: Some(view),
            }
        })
        .collect();
    for e in log {
        if matches!(e.kind.as_str(), "switched" | "rolled-back") {
            continue;
        }
        let k = event_kind(&e.kind);
        items.push(FeedItem {
            key: format!("e-{}", e.id),
            clock: text::clock(&e.at),
            full_time: text::full_time(&e.at),
            provider: e.provider,
            provider_name: provider_name(e.provider),
            category: k.map(|k| k.1).unwrap_or(Category::System),
            title: k.map(|k| k.0.to_string()).unwrap_or_else(|| e.kind.clone()),
            account_name: e.account_id.as_deref().map(|id| account_label(id).unwrap_or_else(|| "Removed account".into())),
            kind: e.kind,
            text: e.message,
            at: e.at,
            switch: None,
        });
    }
    let stamp = |i: &FeedItem| text::parse(&i.at).map(|t| t.timestamp_millis()).unwrap_or(0);
    items.sort_by_key(|i| std::cmp::Reverse(stamp(i)));

    let providers: Vec<ProviderOption> = ProviderId::ALL
        .into_iter()
        .filter(|p| items.iter().any(|i| i.provider == Some(*p)))
        .map(|p| ProviderOption { id: p, name: p.name().into() })
        .collect();

    let range = q.range_days.unwrap_or(DEFAULT_RANGE_DAYS);
    let since = if range <= 0 { i64::MIN } else { (now - chrono::Duration::days(range)).timestamp_millis() };
    let needle = q.query.as_deref().map(str::trim).unwrap_or_default().to_lowercase();
    let in_scope = |i: &FeedItem| stamp(i) >= since && q.provider.is_none_or(|p| i.provider == Some(p));
    let total = items.len();
    let mut counts = FeedCounts::default();
    for i in items.iter().filter(|i| in_scope(i)) {
        counts.all += 1;
        match i.category {
            Category::Switch => counts.switch += 1,
            Category::Account => counts.account += 1,
            Category::Problem => counts.problem += 1,
            Category::System => counts.system += 1,
        }
    }
    let matches = |i: &FeedItem| {
        let sw = i.switch.as_ref();
        needle.is_empty()
            || [
                Some(&i.title),
                Some(&i.text),
                sw.and_then(|s| s.record.from_label.as_ref()),
                sw.and_then(|s| s.record.to_label.as_ref()),
            ]
            .into_iter()
            .flatten()
            .any(|v| v.to_lowercase().contains(&needle))
    };
    let filtered: Vec<FeedItem> = items
        .into_iter()
        .filter(|i| in_scope(i) && q.category.is_none_or(|c| i.category == c) && matches(i))
        .collect();

    let switches: Vec<&SwitchView> = filtered.iter().filter_map(|i| i.switch.as_ref()).collect();
    let ok = switches.iter().filter(|s| s.record.status == "succeeded").count();
    let durations: Vec<i64> = switches
        .iter()
        .filter_map(|s| text::elapsed_ms(&s.record.started_at, s.record.completed_at.as_deref()))
        .collect();
    let avg = (!durations.is_empty()).then(|| durations.iter().sum::<i64>() as f64 / durations.len() as f64);
    let problems = filtered.iter().filter(|i| i.category == Category::Problem).count();
    let range_text = match range {
        r if r <= 0 => "All time".to_string(),
        1 => "Last 24 hours".to_string(),
        r => format!("Last {r} days"),
    };
    let stats = vec![
        FeedStat { label: "Switches".into(), value: switches.len().to_string(), hint: range_text },
        FeedStat {
            label: "Succeeded".into(),
            value: if switches.is_empty() {
                "—".into()
            } else {
                format!("{}%", ((ok * 100) as f64 / switches.len() as f64).round())
            },
            hint: format!("{ok} of {} verified", switches.len()),
        },
        FeedStat {
            label: "Average time".into(),
            value: match avg {
                None => "—".into(),
                Some(a) if a < 1000.0 => format!("{} ms", a.round()),
                Some(a) => format!("{:.1} s", a / 1000.0),
            },
            hint: "Start to verified".into(),
        },
        FeedStat {
            label: "Needs a look".into(),
            value: problems.to_string(),
            hint: switches
                .first()
                .map(|s| format!("Last switch {}", s.started_text))
                .unwrap_or_else(|| "No switches yet".into()),
        },
    ];

    let mut groups: Vec<FeedGroup> = Vec::new();
    for i in filtered {
        let day = text::day_label(&i.at, now);
        match groups.last_mut() {
            Some(g) if g.day == day => g.items.push(i),
            _ => groups.push(FeedGroup { day, items: vec![i] }),
        }
    }
    Feed {
        groups,
        counts,
        stats,
        providers,
        total,
        filtering: !needle.is_empty() || q.provider.is_some() || q.category.is_some() || range != DEFAULT_RANGE_DAYS,
    }
}

pub fn unsupported_usage_notice(states: &[(&ProviderInfo, usize)]) -> Option<Notice> {
    let names: Vec<&str> =
        states.iter().filter(|(i, n)| *n > 0 && !usage_supported(i)).map(|(i, _)| i.name.as_str()).collect();
    if names.is_empty() {
        return None;
    }
    let joined = names.join(" and ");
    Some(Notice {
        text: format!("Usage isn't available for {joined}"),
        detail: format!(
            "{joined} {} report usage limits, so AltFlip can't show them. Switching accounts still works.",
            if names.len() == 1 { "doesn't" } else { "don't" }
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, p: ProviderId, status: &str, started: &str) -> SwitchRecord {
        SwitchRecord {
            id: id.into(),
            provider: p,
            from_account_id: None,
            to_account_id: None,
            from_label: Some("Work".into()),
            to_label: Some("Personal".into()),
            status: status.into(),
            switch_mode: "new-sessions".into(),
            error_code: None,
            message: None,
            started_at: started.into(),
            completed_at: Some(started.replace(":00.000Z", ":01.500Z")),
        }
    }

    #[test]
    fn undo_only_latest_success_per_provider() {
        let h = vec![
            rec("c", ProviderId::Claude, "recovered", "2026-01-03T00:00:00.000Z"),
            rec("b", ProviderId::Claude, "succeeded", "2026-01-02T00:00:00.000Z"),
            rec("x", ProviderId::Codex, "rolled-back", "2026-01-02T00:00:00.000Z"),
            rec("y", ProviderId::Codex, "succeeded", "2026-01-01T00:00:00.000Z"),
        ];
        let ids = undoable_ids(&h);
        assert!(!ids.contains("b"));
        assert!(ids.contains("y"));
    }

    #[test]
    fn feed_filters_counts_and_groups() {
        let now = text::parse("2026-01-03T12:00:00Z").unwrap();
        let h = vec![
            rec("a", ProviderId::Claude, "succeeded", "2026-01-03T10:00:00.000Z"),
            rec("b", ProviderId::Codex, "rolled-back", "2026-01-02T10:00:00.000Z"),
        ];
        let log = vec![
            ActivityRow {
                id: 1,
                at: "2026-01-03T09:00:00.000Z".into(),
                provider: Some(ProviderId::Claude),
                account_id: Some("gone".into()),
                kind: "added".into(),
                message: "Saved Work".into(),
            },
            ActivityRow {
                id: 2,
                at: "2026-01-03T09:30:00.000Z".into(),
                provider: Some(ProviderId::Claude),
                account_id: None,
                kind: "switched".into(),
                message: "dup".into(),
            },
        ];
        let f = feed(h.clone(), log.clone(), |_| None, &FeedQuery::default(), now);
        assert_eq!(f.total, 3);
        assert_eq!((f.counts.all, f.counts.switch, f.counts.problem, f.counts.account), (3, 1, 1, 1));
        assert_eq!(f.stats[0].value, "2");
        assert_eq!(f.stats[1].value, "50%");
        assert_eq!(f.stats[2].value, "1.5 s");
        assert!(!f.filtering);
        let first = &f.groups[0].items[0];
        assert_eq!(first.title, "Work → Personal");
        assert!(first.switch.as_ref().unwrap().undoable);
        let added = f.groups.iter().flat_map(|g| &g.items).find(|i| i.kind == "added").unwrap();
        assert_eq!(added.account_name.as_deref(), Some("Removed account"));

        let q = FeedQuery { provider: Some(ProviderId::Codex), ..Default::default() };
        let f = feed(h.clone(), log.clone(), |_| None, &q, now);
        assert_eq!(f.counts.all, 1);
        assert!(f.filtering);

        let q = FeedQuery { query: Some("saved".into()), ..Default::default() };
        let f = feed(h, log, |_| None, &q, now);
        assert_eq!(f.groups.iter().map(|g| g.items.len()).sum::<usize>(), 1);
        assert_eq!(f.counts.all, 3);
    }

    #[test]
    fn usage_view_after_reset() {
        let now = text::parse("2026-01-01T10:00:00Z").unwrap();
        let row = UsageRow {
            account_id: "a".into(),
            provider: ProviderId::Codex,
            retrieved_at: "2026-01-01T08:00:00Z".into(),
            reading: altflip_core::UsageReading {
                usage_type: "primary".into(),
                label: "5-hour usage".into(),
                used_percent: Some(72.4),
                used_value: None,
                limit_value: None,
                window_minutes: Some(300),
                reset_at: Some("2026-01-01T09:00:00Z".into()),
                source: "Codex session log".into(),
                confidence: "provider-reported".into(),
                observed_at: "2026-01-01T08:00:00Z".into(),
                account_key: None,
            },
        };
        let v = usage_view(row.clone(), now);
        assert!(v.reset_passed);
        assert_eq!(v.percent, Some(0));
        assert_eq!(v.left_text, "New window started");
        let mut fresh = row;
        fresh.reading.reset_at = Some("2026-01-01T12:30:00Z".into());
        let v = usage_view(fresh, now);
        assert_eq!(v.percent, Some(72));
        assert_eq!(v.left_text, "28% left");
        assert_eq!(v.reset_text.as_deref(), Some("Resets in 2h 30m"));
        let inline = inline_usage(&[v], now).unwrap();
        assert_eq!(inline.text, "72% · 2h 30m");
        assert_eq!(inline.tone, Tone::Warn);
    }
}
