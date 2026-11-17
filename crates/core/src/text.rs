use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "bindings.ts")]
pub enum Tone {
    Ok,
    Warn,
    Danger,
    Neutral,
    Info,
    Accent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "bindings.ts")]
pub struct Badge {
    pub text: String,
    pub tone: Tone,
}

impl Badge {
    pub fn new(text: impl Into<String>, tone: Tone) -> Self {
        Badge { text: text.into(), tone }
    }
}

pub fn parse(iso: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(iso).ok().map(|d| d.with_timezone(&Utc))
}

pub fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

pub fn relative_time(iso: Option<&str>, now: DateTime<Utc>) -> String {
    let Some(iso) = iso else { return "never".into() };
    let Some(t) = parse(iso) else { return "unknown".into() };
    let s = ((now - t).num_milliseconds() as f64 / 1000.0).round() as i64;
    let abs = s.unsigned_abs() as f64;
    if abs < 45.0 {
        return if s >= 0 { "just now".into() } else { "in a moment".into() };
    }
    let fmt = |n: f64, unit: &str| plural(n.round() as usize, unit);
    let out = if abs < 3600.0 {
        fmt(abs / 60.0, "min")
    } else if abs < 86400.0 {
        fmt(abs / 3600.0, "hour")
    } else {
        fmt(abs / 86400.0, "day")
    };
    if s >= 0 {
        format!("{out} ago")
    } else {
        format!("in {out}")
    }
}

pub fn countdown(iso: Option<&str>, now: DateTime<Utc>) -> Option<String> {
    let t = parse(iso?)?;
    let ms = (t - now).num_milliseconds();
    if ms <= 0 {
        return Some("now".into());
    }
    let m = ms / 60000;
    let (d, h, mm) = (m / 1440, (m % 1440) / 60, m % 60);
    Some(if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {mm}m")
    } else {
        format!("{mm}m")
    })
}

fn local(iso: &str) -> Option<DateTime<Local>> {
    parse(iso).map(|t| t.with_timezone(&Local))
}

pub fn clock(iso: &str) -> String {
    local(iso).map(|t| t.format("%-I:%M %p").to_string()).unwrap_or_default()
}

pub fn full_time(iso: &str) -> String {
    local(iso).map(|t| t.format("%b %-d, %Y, %-I:%M:%S %p").to_string()).unwrap_or_else(|| iso.to_string())
}

pub fn day_label(iso: &str, now: DateTime<Utc>) -> String {
    let Some(t) = local(iso) else { return String::new() };
    let today = now.with_timezone(&Local).date_naive();
    let day = t.date_naive();
    if day == today {
        "Today".into()
    } else if today.pred_opt() == Some(day) {
        "Yesterday".into()
    } else {
        t.format("%a, %b %-d").to_string()
    }
}

pub fn reset_text(iso: Option<&str>, now: DateTime<Utc>) -> Option<String> {
    let iso = iso?;
    let t = parse(iso)?;
    if (t - now).num_milliseconds() < 86_400_000 {
        return Some(format!("Resets in {}", countdown(Some(iso), now)?));
    }
    Some(format!("Resets {}", t.with_timezone(&Local).format("%a %-I:%M %p")))
}

pub fn duration_text(ms: i64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else if ms < 10_000 {
        format!("{:.1} s", ms as f64 / 1000.0)
    } else {
        format!("{:.0} s", ms as f64 / 1000.0)
    }
}

pub fn elapsed_ms(start: &str, end: Option<&str>) -> Option<i64> {
    let ms = (parse(end?)? - parse(start)?).num_milliseconds();
    (ms >= 0).then_some(ms)
}

pub fn plan_label(plan: Option<&str>) -> Option<String> {
    let plan = plan.filter(|p| !p.is_empty())?;
    let known = match plan.to_lowercase().as_str() {
        "pro" => "Pro",
        "max" => "Max",
        "team" => "Team",
        "enterprise" => "Enterprise",
        "plus" => "Plus",
        "free" => "Free",
        "api" => "API",
        "stripe_subscription" => "Subscription",
        _ => "",
    };
    if !known.is_empty() {
        return Some(known.into());
    }
    let spaced = plan.replace(['_', '-'], " ");
    let mut out = String::with_capacity(spaced.len());
    let mut start = true;
    for c in spaced.chars() {
        if start && c.is_alphanumeric() {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
        start = !c.is_alphanumeric();
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> DateTime<Utc> {
        parse(s).unwrap()
    }

    #[test]
    fn countdowns() {
        let now = at("2026-01-01T00:00:00Z");
        assert_eq!(countdown(Some("2026-01-01T03:15:00Z"), now).as_deref(), Some("3h 15m"));
        assert_eq!(countdown(Some("2026-01-03T01:00:00Z"), now).as_deref(), Some("2d 1h"));
        assert_eq!(countdown(Some("2025-12-31T00:00:00Z"), now).as_deref(), Some("now"));
        assert_eq!(countdown(None, now), None);
    }

    #[test]
    fn relative_times() {
        let now = at("2026-01-01T01:00:00Z");
        assert_eq!(relative_time(Some("2026-01-01T00:00:00Z"), now), "1 hour ago");
        assert_eq!(relative_time(Some("2026-01-01T00:59:50Z"), now), "just now");
        assert_eq!(relative_time(Some("2026-01-01T01:10:00Z"), now), "in 10 mins");
        assert_eq!(relative_time(None, now), "never");
        assert_eq!(relative_time(Some("nope"), now), "unknown");
    }

    #[test]
    fn plans_and_durations() {
        assert_eq!(plan_label(Some("max")).as_deref(), Some("Max"));
        assert_eq!(plan_label(Some("some_tier")).as_deref(), Some("Some Tier"));
        assert_eq!(plan_label(None), None);
        assert_eq!(duration_text(420), "420 ms");
        assert_eq!(duration_text(2345), "2.3 s");
        assert_eq!(duration_text(12_600), "13 s");
        assert_eq!(elapsed_ms("2026-01-01T00:00:00Z", Some("2026-01-01T00:00:01.5Z")), Some(1500));
        assert_eq!(elapsed_ms("2026-01-01T00:00:01Z", Some("2026-01-01T00:00:00Z")), None);
    }
}
