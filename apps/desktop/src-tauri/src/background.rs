use crate::{notify, tray, AppState};
use altflip_core::{AuthHealth, Result};
use altflip_engine::UsageRow;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const STATUS_EVERY: Duration = Duration::from_secs(300);
const FOCUS_THROTTLE: Duration = Duration::from_secs(15);

fn refresh_status(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut t) = state.last_refresh.lock() {
        *t = Some(Instant::now());
    }
    let Ok(o) = state.engine.overview(true) else { return };
    let _ = app.emit("altflip://overview", &o);
    tray::refresh(app, &o);
    let Ok(settings) = state.engine.settings() else { return };
    if !(settings.notifications && settings.notify_auth) {
        return;
    }
    for p in &o.providers {
        for a in &p.accounts {
            if matches!(a.row.health, AuthHealth::Expired | AuthHealth::NeedsReauth) {
                let key = format!("auth:{}:{}", a.row.id, a.row.health.as_str());
                if state.notified.lock().map(|mut n| n.insert(key)).unwrap_or(false) {
                    notify(
                        app,
                        "Sign-in needed",
                        &format!("Your {} {} account needs authentication.", p.info.name, a.row.label),
                        true,
                    );
                }
            }
        }
    }
}

pub fn refresh_soon(app: &AppHandle) {
    let state = app.state::<AppState>();
    let due = state.last_refresh.lock().ok().and_then(|t| *t).is_none_or(|t| t.elapsed() > FOCUS_THROTTLE);
    if due {
        let app = app.clone();
        std::thread::spawn(move || refresh_status(&app));
    }
}

pub fn refresh_usage(app: &AppHandle) -> Result<Vec<UsageRow>> {
    let state = app.state::<AppState>();
    let rows = state.engine.refresh_usage()?;
    let _ = app.emit("altflip://usage", &rows);
    let settings = state.engine.settings()?;
    if settings.notifications && settings.notify_usage {
        let accounts = state.engine.account_views(None).unwrap_or_default();
        for r in &rows {
            let Some(pct) = r.reading.used_percent else { continue };
            if pct < settings.usage_threshold as f64 {
                continue;
            }
            let key = format!(
                "usage:{}:{}:{}",
                r.account_id,
                r.reading.usage_type,
                r.reading.reset_at.clone().unwrap_or_default()
            );
            if !state.notified.lock().map(|mut n| n.insert(key)).unwrap_or(false) {
                continue;
            }
            let label =
                accounts.iter().find(|a| a.row.id == r.account_id).map(|a| a.row.label.clone()).unwrap_or_default();
            let msg = if pct >= 100.0 {
                format!("{} {label} has used its {}.", r.provider, r.reading.label.to_lowercase())
            } else {
                format!("{} {label} reached {pct:.0}% of its {}.", r.provider, r.reading.label.to_lowercase())
            };
            notify(app, "Usage warning", &msg, true);
        }
    }
    Ok(rows)
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        refresh_status(&app);
        let _ = refresh_usage(&app);
        let mut last_usage = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(30));
            let state = app.state::<AppState>();
            let status_due = state.last_refresh.lock().ok().and_then(|t| *t).is_none_or(|t| t.elapsed() > STATUS_EVERY);
            if status_due {
                refresh_status(&app);
            }
            let Ok(settings) = state.engine.settings() else { continue };
            if settings.usage_monitoring
                && last_usage.elapsed() > Duration::from_secs(settings.usage_refresh_minutes * 60)
            {
                last_usage = Instant::now();
                let _ = refresh_usage(&app);
            }
        }
    });
}
