use chrono::{DateTime, Local, Locale, TimeZone, Utc};
use std::fmt::Display;
use std::sync::OnceLock;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub locale: Locale,
    pub h24: bool,
}

pub fn region() -> Region {
    static REGION: OnceLock<Region> = OnceLock::new();
    *REGION.get_or_init(|| {
        let locale = sys_locale::get_locale().as_deref().and_then(parse_locale).unwrap_or(Locale::en_US);
        Region { locale, h24: system_hour_cycle().unwrap_or_else(|| uses_24h(locale)) }
    })
}

pub fn parse_locale(tag: &str) -> Option<Locale> {
    let base = tag.split(['@', '.']).next()?.replace('-', "_");
    Locale::try_from(base.as_str()).ok().or_else(|| {
        let lang = base.split('_').next()?.to_lowercase();
        Locale::try_from(format!("{lang}_{}", lang.to_uppercase()).as_str()).ok()
    })
}

pub fn uses_24h(locale: Locale) -> bool {
    Utc.with_ymd_and_hms(2000, 1, 1, 13, 0, 0)
        .single()
        .is_some_and(|t| t.format_localized("%X", locale).to_string().contains("13"))
}

#[cfg(target_os = "macos")]
fn system_hour_cycle() -> Option<bool> {
    let read = |key: &str| {
        std::process::Command::new("/usr/bin/defaults")
            .args(["read", "-g", key])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .is_some_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
    };
    if read("AppleICUForce24HourTime") {
        Some(true)
    } else if read("AppleICUForce12HourTime") {
        Some(false)
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
fn system_hour_cycle() -> Option<bool> {
    None
}

fn local(iso: &str) -> Option<DateTime<Local>> {
    parse(iso).map(|t| t.with_timezone(&Local))
}

pub fn clock_in<Tz: TimeZone>(t: &DateTime<Tz>, r: Region, seconds: bool) -> String
where
    Tz::Offset: Display,
{
    let fmt = match (r.h24, seconds) {
        (true, false) => "%H:%M",
        (true, true) => "%H:%M:%S",
        (false, false) => "%-I:%M %p",
        (false, true) => "%-I:%M:%S %p",
    };
    t.format_localized(fmt, r.locale).to_string()
}

pub fn date_in<Tz: TimeZone>(t: &DateTime<Tz>, r: Region) -> String
where
    Tz::Offset: Display,
{
    t.format_localized("%x", r.locale).to_string()
}

pub fn weekday_in<Tz: TimeZone>(t: &DateTime<Tz>, r: Region) -> String
where
    Tz::Offset: Display,
{
    t.format_localized("%a", r.locale).to_string()
}

pub fn clock(iso: &str) -> String {
    local(iso).map(|t| clock_in(&t, region(), false)).unwrap_or_default()
}

pub fn full_time(iso: &str) -> String {
    let r = region();
    local(iso).map(|t| format!("{}, {}", date_in(&t, r), clock_in(&t, r, true))).unwrap_or_else(|| iso.to_string())
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
        let r = region();
        format!("{} {}", weekday_in(&t, r), date_in(&t, r))
    }
}

pub fn reset_text(iso: Option<&str>, now: DateTime<Utc>) -> Option<String> {
    let iso = iso?;
    let t = parse(iso)?;
    if (t - now).num_milliseconds() < 86_400_000 {
        return Some(format!("Resets in {}", countdown(Some(iso), now)?));
    }
    let t = t.with_timezone(&Local);
    let r = region();
    Some(format!("Resets {} {}", weekday_in(&t, r), clock_in(&t, r, false)))
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
    fn locale_formats() {
        assert_eq!(parse_locale("en-GB"), Some(Locale::en_GB));
        assert_eq!(parse_locale("en_US@rg=bdzzzz"), Some(Locale::en_US));
        assert_eq!(parse_locale("de"), Some(Locale::de_DE));
        assert!(!uses_24h(Locale::en_US));
        assert!(uses_24h(Locale::de_DE));
        assert!(uses_24h(Locale::en_GB));
        let t = at("2026-01-02T13:05:09Z");
        let us = Region { locale: Locale::en_US, h24: false };
        let de = Region { locale: Locale::de_DE, h24: true };
        assert_eq!(clock_in(&t, us, false), "1:05 PM");
        assert_eq!(clock_in(&t, us, true), "1:05:09 PM");
        assert_eq!(clock_in(&t, de, false), "13:05");
        assert_eq!(date_in(&t, us), "01/02/2026");
        assert_eq!(date_in(&t, de), "02.01.2026");
        assert_eq!(weekday_in(&t, de), "Fr");
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
