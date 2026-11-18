use crate::{background, notify, show_main, tray, AppState};
use altflip_core::{Error, ProviderId, Result};
use altflip_core::error::ApiError;
use altflip_engine::{
    AccountPatch, AccountRow, Feed, FeedQuery, Notice, Overview, Progress, RecoveryAction, Settings,
    SwitchOptions, SwitchOutcome, SwitchPlan, SwitchView,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| Error::Platform(format!("Background task failed: {e}")))?
}

fn provider(id: &str) -> Result<ProviderId> {
    ProviderId::parse(id).ok_or_else(|| Error::Invalid(format!("Unknown provider {id}")))
}

pub fn broadcast(app: &AppHandle) {
    broadcast_with(app, false);
}

pub fn broadcast_with(app: &AppHandle, refresh_live: bool) {
    let state = app.state::<AppState>();
    if let Ok(o) = state.engine.overview(refresh_live) {
        let _ = app.emit("altflip://overview", &o);
        tray::refresh(app, &o);
    }
}

#[tauri::command]
pub async fn get_overview(state: State<'_, AppState>, app: AppHandle, refresh: bool) -> Result<Overview> {
    let engine = state.engine.clone();
    let o = blocking(move || engine.overview(refresh)).await?;
    tray::refresh(&app, &o);
    Ok(o)
}

#[tauri::command]
pub async fn plan_switch(state: State<'_, AppState>, account_id: String) -> Result<SwitchPlan> {
    let engine = state.engine.clone();
    blocking(move || engine.plan_switch(&account_id)).await
}

pub fn run_switch(app: &AppHandle, account_id: &str, confirm_restart: bool, source: &str) -> Result<SwitchOutcome> {
    let state = app.state::<AppState>();
    let emitter = app.clone();
    let progress = move |p: Progress| {
        let _ = emitter.emit("altflip://switch-progress", &p);
    };
    tray::set_busy(app, true);
    let res = state.engine.switch(account_id, SwitchOptions { confirm_restart }, &progress);
    tray::set_busy(app, false);
    if let Ok(provider) = state.engine.account(account_id).map(|a| a.provider) {
        let _ = app.emit(
            "altflip://switch-progress",
            &Progress { provider, account_id: account_id.to_string(), stage: "done".into(), message: String::new() },
        );
    }
    let settings = state.engine.settings().ok();
    let enabled = settings.as_ref().is_some_and(|s| s.notifications);
    let background = source != "window";
    match &res {
        Ok(o) if o.status == "succeeded" && enabled && settings.as_ref().is_some_and(|s| s.notify_switch) => {
            let mut body = o.message.clone();
            if !o.restarted.is_empty() {
                body.push_str(&format!(". Reopened {}", o.restarted.join(", ")));
            }
            notify(app, "Account switched", &body, background);
        }
        Err(Error::ConfirmationRequired(_)) | Err(Error::Busy(_)) => {}
        Err(e) if enabled => notify(app, "Switch failed", &e.to_string(), background),
        _ => {}
    }
    broadcast_with(app, true);
    res
}

#[tauri::command]
pub async fn switch_account(app: AppHandle, account_id: String, confirm_restart: bool) -> Result<SwitchOutcome> {
    blocking(move || run_switch(&app, &account_id, confirm_restart, "window")).await
}

#[tauri::command]
pub async fn save_current(
    state: State<'_, AppState>,
    app: AppHandle,
    provider_id: String,
    label: Option<String>,
    category: Option<String>,
) -> Result<AccountRow> {
    let engine = state.engine.clone();
    let p = provider(&provider_id)?;
    let row = blocking(move || engine.save_current(p, label, category)).await?;
    broadcast(&app);
    Ok(row)
}

#[derive(Serialize, Clone, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct LoginLine {
    login_id: String,
    line: String,
}

#[derive(Serialize, Clone, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct LoginDone {
    login_id: String,
    account: Option<AccountRow>,
    error: Option<ApiError>,
}

#[tauri::command]
pub fn start_login(
    state: State<'_, AppState>,
    app: AppHandle,
    provider_id: String,
    label: Option<String>,
    category: Option<String>,
) -> Result<String> {
    let p = provider(&provider_id)?;
    if !state.engine.provider(p)?.capabilities().isolated_login {
        return Err(Error::Unsupported(format!("{p} does not support signing in from AltFlip")));
    }
    let login_id = altflip_core::new_id();
    let engine = state.engine.clone();
    let id = login_id.clone();
    std::thread::spawn(move || {
        let out_app = app.clone();
        let out_id = id.clone();
        let out = move |line: &str| {
            let _ =
                out_app.emit("altflip://login-output", LoginLine { login_id: out_id.clone(), line: line.to_string() });
        };
        let (account, error) = match engine.login_new(p, &id, label, category, &out) {
            Ok(a) => (Some(a), None),
            Err(e) => (None, Some(ApiError { code: e.code().into(), message: e.to_string() })),
        };
        let _ = app.emit("altflip://login-done", LoginDone { login_id: id, account, error });
        broadcast(&app);
    });
    Ok(login_id)
}

#[tauri::command]
pub fn cancel_login(state: State<'_, AppState>, login_id: String) {
    state.engine.cancel_login(&login_id);
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
    patch: AccountPatch,
) -> Result<AccountRow> {
    let engine = state.engine.clone();
    let row = blocking(move || engine.update_account(&id, patch)).await?;
    broadcast(&app);
    Ok(row)
}

#[tauri::command]
pub async fn remove_account(state: State<'_, AppState>, app: AppHandle, id: String) -> Result<()> {
    let engine = state.engine.clone();
    blocking(move || engine.remove_account(&id)).await?;
    broadcast(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_history(state: State<'_, AppState>, limit: Option<i64>) -> Result<Vec<SwitchView>> {
    let engine = state.engine.clone();
    blocking(move || engine.history_views(limit.unwrap_or(100).clamp(1, 500))).await
}

#[tauri::command]
pub async fn activity_feed(state: State<'_, AppState>, query: FeedQuery) -> Result<Feed> {
    let engine = state.engine.clone();
    blocking(move || engine.activity_feed(&query)).await
}

#[tauri::command]
pub async fn move_account(state: State<'_, AppState>, app: AppHandle, id: String, delta: i64) -> Result<()> {
    let engine = state.engine.clone();
    blocking(move || engine.move_account(&id, delta)).await?;
    broadcast(&app);
    Ok(())
}

#[tauri::command]
pub async fn resolve_incomplete(
    state: State<'_, AppState>,
    app: AppHandle,
    operation_id: String,
    action: RecoveryAction,
) -> Result<()> {
    let engine = state.engine.clone();
    blocking(move || engine.resolve_incomplete(&operation_id, action)).await?;
    broadcast(&app);
    Ok(())
}

#[tauri::command]
pub async fn undo_switch(state: State<'_, AppState>, app: AppHandle, operation_id: String) -> Result<()> {
    let engine = state.engine.clone();
    blocking(move || engine.undo_switch(&operation_id)).await?;
    broadcast(&app);
    Ok(())
}

#[tauri::command]
pub async fn refresh_usage(app: AppHandle, provider_id: Option<String>) -> Result<Option<Notice>> {
    let only = provider_id.as_deref().map(provider).transpose()?;
    blocking(move || {
        background::refresh_usage_with(&app, true, only)?;
        Ok(app.state::<AppState>().engine.usage_notice())
    })
    .await
}

#[tauri::command]
pub async fn claude_status_line(state: State<'_, AppState>) -> Result<altflip_engine::StatusLineState> {
    let engine = state.engine.clone();
    blocking(move || engine.claude_status_line()).await
}

#[tauri::command]
pub async fn set_claude_status_line(state: State<'_, AppState>, on: bool) -> Result<altflip_engine::StatusLineState> {
    let engine = state.engine.clone();
    blocking(move || engine.set_claude_status_line(on)).await
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    state.engine.settings()
}

#[tauri::command]
pub fn set_setting(app: AppHandle, key: String, value: serde_json::Value) -> Result<Settings> {
    let state = app.state::<AppState>();
    let previous = state.engine.settings()?;
    let settings = state.engine.set_setting(&key, value)?;
    if let Err(e) = crate::apply_setting(&app, &key, &settings) {
        let prior = serde_json::to_value(&previous)?.get(&key).cloned().unwrap_or(serde_json::Value::Null);
        let _ = state.engine.set_setting(&key, prior);
        return Err(e);
    }
    let _ = app.emit("altflip://settings", &settings);
    if key == "experimentalAntigravity" {
        broadcast(&app);
    }
    Ok(settings)
}

#[tauri::command]
pub fn reset_settings(app: AppHandle) -> Result<Settings> {
    let state = app.state::<AppState>();
    let s = state.engine.reset_settings()?;
    for key in ["launchAtLogin", "showDockIcon", "globalShortcut"] {
        let _ = crate::apply_setting(&app, key, &s);
    }
    let _ = app.emit("altflip://settings", &s);
    Ok(s)
}

#[tauri::command]
pub async fn export_backup(state: State<'_, AppState>, path: String, passphrase: String) -> Result<usize> {
    let engine = state.engine.clone();
    blocking(move || engine.export_backup(std::path::Path::new(&path), &passphrase)).await
}

#[tauri::command]
pub async fn import_backup(
    state: State<'_, AppState>,
    app: AppHandle,
    path: String,
    passphrase: String,
) -> Result<usize> {
    let engine = state.engine.clone();
    let n = blocking(move || engine.import_backup(std::path::Path::new(&path), &passphrase)).await?;
    broadcast(&app);
    Ok(n)
}

#[tauri::command]
pub async fn diagnostics(state: State<'_, AppState>) -> Result<String> {
    let engine = state.engine.clone();
    let v = blocking(move || engine.diagnostics()).await?;
    Ok(serde_json::to_string_pretty(&v)?)
}

#[derive(Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    os: String,
    data_dir: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>, app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS.into(),
        data_dir: state.engine.data_dir().display().to_string(),
    }
}

#[tauri::command]
pub fn show_main_window(app: AppHandle, page: Option<String>) {
    show_main(&app, page.as_deref());
}

#[tauri::command]
pub fn hide_quick(app: AppHandle) {
    crate::hide_quick(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod bindings {
    use super::*;
    use altflip_core::{ProviderId, UsageReading};
    use altflip_engine::StatusLineState;

    #[test]
    fn export_bindings() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib");
        let _ = std::fs::remove_file(dir.join("bindings.ts"));
        let cfg = ts_rs::Config::new().with_out_dir(&dir).with_large_int("number");
        Overview::export_all(&cfg).unwrap();
        Settings::export_all(&cfg).unwrap();
        SwitchPlan::export_all(&cfg).unwrap();
        SwitchOutcome::export_all(&cfg).unwrap();
        Progress::export_all(&cfg).unwrap();
        AccountPatch::export_all(&cfg).unwrap();
        Feed::export_all(&cfg).unwrap();
        FeedQuery::export_all(&cfg).unwrap();
        Notice::export_all(&cfg).unwrap();
        StatusLineState::export_all(&cfg).unwrap();
        AppInfo::export_all(&cfg).unwrap();
        LoginLine::export_all(&cfg).unwrap();
        LoginDone::export_all(&cfg).unwrap();
        RecoveryAction::export_all(&cfg).unwrap();
        ApiError::export_all(&cfg).unwrap();
        ProviderId::export_all(&cfg).unwrap();
        UsageReading::export_all(&cfg).unwrap();
        assert!(dir.join("bindings.ts").is_file());
    }
}
