mod background;
mod commands;
mod tray;

use altflip_core::secrets::SystemSecrets;
use altflip_core::{Error, Result};
use altflip_engine::{Engine, EngineConfig, Settings};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Rect, WindowEvent};

pub struct AppState {
    pub engine: Arc<Engine>,
    pub notified: Mutex<HashSet<String>>,
    pub last_refresh: Mutex<Option<Instant>>,
    pub quick_hidden_at: Mutex<Option<Instant>>,
    pub tray_key: Mutex<String>,
}

#[cfg(target_os = "macos")]
fn set_dock(app: &AppHandle, visible: bool) {
    let policy = if visible { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory };
    let _ = app.set_activation_policy(policy);
    if visible {
        use objc2::AnyThread;
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::{MainThreadMarker, NSData};
        if let Some(mtm) = MainThreadMarker::new() {
            let ns_app = NSApplication::sharedApplication(mtm);
            let data = NSData::with_bytes(include_bytes!("../icons/icon.png"));
            if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                unsafe {
                    ns_app.setApplicationIconImage(Some(&image));
                }
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn set_dock(_app: &AppHandle, _visible: bool) {}

fn refresh_dock(app: &AppHandle) {
    let always = app.state::<AppState>().engine.settings().map(|s| s.show_dock_icon).unwrap_or(false);
    let main_open = app.get_webview_window("main").and_then(|w| w.is_visible().ok()).unwrap_or(false);
    set_dock(app, always || main_open);
}

pub(crate) fn show_main(app: &AppHandle, page: Option<&str>) {
    hide_quick(app);
    if let Some(w) = app.get_webview_window("main") {
        set_dock(app, true);
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        if let Some(p) = page {
            let _ = app.emit_to("main", "altflip://navigate", p);
        }
    }
}

pub(crate) fn hide_quick(app: &AppHandle) {
    if let Some(q) = app.get_webview_window("quick") {
        if q.is_visible().unwrap_or(false) {
            let _ = q.hide();
            if let Ok(mut t) = app.state::<AppState>().quick_hidden_at.lock() {
                *t = Some(Instant::now());
            }
        }
    }
}

fn apply_glass(app: &AppHandle) {
    use tauri::window::{Effect, EffectState, EffectsBuilder};
    for (label, radius) in [("main", 0.0), ("quick", 18.0)] {
        let Some(w) = app.get_webview_window(label) else { continue };
        let effect = if cfg!(target_os = "macos") {
            Some(if label == "quick" { Effect::Popover } else { Effect::Sidebar })
        } else if cfg!(target_os = "windows") && label == "main" {
            Some(Effect::Mica)
        } else {
            None
        };
        if let Some(effect) = effect {
            let mut b = EffectsBuilder::new().effect(effect).state(EffectState::FollowsWindowActiveState);
            if radius > 0.0 {
                b = b.radius(radius);
            }
            let _ = w.set_effects(b.build());
        }
    }
}

pub(crate) fn toggle_quick(app: &AppHandle, anchor: Option<Rect>) {
    let Some(w) = app.get_webview_window("quick") else { return };
    if w.is_visible().unwrap_or(false) {
        hide_quick(app);
        return;
    }
    let recently = app
        .state::<AppState>()
        .quick_hidden_at
        .lock()
        .ok()
        .and_then(|t| *t)
        .is_some_and(|t| t.elapsed() < Duration::from_millis(250));
    if recently {
        return;
    }
    let scale = w.scale_factor().unwrap_or(1.0);
    let win = w.outer_size().unwrap_or_default().to_logical::<f64>(scale);
    match anchor.or_else(|| app.tray_by_id(tray::TRAY_ID).and_then(|t| t.rect().ok().flatten())) {
        Some(rect) => {
            let pos = rect.position.to_logical::<f64>(scale);
            let size = rect.size.to_logical::<f64>(scale);
            let mut x = pos.x + size.width / 2.0 - win.width / 2.0;
            let mut y = if cfg!(target_os = "macos") { pos.y + size.height + 4.0 } else { pos.y - win.height - 8.0 };
            if let Ok(Some(m)) = w.current_monitor() {
                let mp = m.work_area().position.to_logical::<f64>(scale);
                let ms = m.work_area().size.to_logical::<f64>(scale);
                x = x.clamp(mp.x + 8.0, (mp.x + ms.width - win.width - 8.0).max(mp.x + 8.0));
                y = y.clamp(mp.y + 4.0, (mp.y + ms.height - win.height - 4.0).max(mp.y + 4.0));
            }
            let _ = w.set_position(LogicalPosition::new(x, y));
        }
        None => {
            let _ = w.center();
        }
    }
    let _ = app.emit_to("quick", "altflip://quick-shown", ());
    let _ = w.show();
    let _ = w.set_focus();
}

pub(crate) fn notify(app: &AppHandle, title: &str, body: &str, background: bool) {
    if !background {
        return;
    }
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

fn apply_shortcut(app: &AppHandle, accel: Option<&str>) -> Result<()> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    match accel.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => gs.register(a).map_err(|e| Error::Invalid(format!("Couldn't use {a} as a shortcut: {e}"))),
        None => Ok(()),
    }
}

pub(crate) fn apply_setting(app: &AppHandle, key: &str, s: &Settings) -> Result<()> {
    match key {
        "launchAtLogin" => {
            use tauri_plugin_autostart::ManagerExt;
            let l = app.autolaunch();
            let r = if s.launch_at_login { l.enable() } else { l.disable() };
            r.map_err(|e| Error::Platform(format!("Could not change launch at login: {e}")))
        }
        "showDockIcon" => {
            refresh_dock(app);
            Ok(())
        }
        "globalShortcut" => apply_shortcut(app, s.global_shortcut.as_deref()),
        _ => Ok(()),
    }
}

fn data_dir(app: &tauri::App) -> std::result::Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        if let Some(d) = std::env::var_os("ALTFLIP_DATA_DIR") {
            return Ok(d.into());
        }
    }
    let dir = app.path().app_data_dir()?;
    migrate_legacy_dir(&dir);
    Ok(dir)
}

fn migrate_legacy_dir(dir: &std::path::Path) {
    if dir.exists() {
        return;
    }
    let Some(parent) = dir.parent() else { return };
    let legacy = parent.join("dev.altflip.app");
    if legacy.join("database").join("manager.db").is_file() {
        if let Err(e) = std::fs::rename(&legacy, dir) {
            eprintln!("altflip: could not move {} to {}: {e}", legacy.display(), dir.display());
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app, None)));
    let app = builder
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["quick"])
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE | tauri_plugin_window_state::StateFlags::POSITION,
                )
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _s, e| {
                    if e.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        toggle_quick(app, None);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            apply_glass(app.handle());
            let dir = data_dir(app)?;
            let home = altflip_core::home_dir()?;
            let engine =
                match Engine::open(EngineConfig { data_dir: dir.clone(), home, secrets: Arc::new(SystemSecrets) }) {
                    Ok(e) => Arc::new(e),
                    Err(e) => {
                        use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                        app.dialog()
                            .message(format!("AltFlip could not open its data at {}.\n\n{e}", dir.display()))
                            .title("AltFlip cannot start")
                            .kind(MessageDialogKind::Error)
                            .blocking_show();
                        return Err(Box::new(e));
                    }
                };
            let pending = engine.recover_on_launch().unwrap_or_default();
            let settings = engine.settings()?;
            app.manage(AppState {
                engine,
                notified: Mutex::new(HashSet::new()),
                last_refresh: Mutex::new(None),
                quick_hidden_at: Mutex::new(None),
                tray_key: Mutex::new(String::new()),
            });
            let handle = app.handle().clone();
            tray::build(&handle)?;
            if let Some(a) = settings.global_shortcut.as_deref() {
                if let Err(e) = apply_shortcut(&handle, Some(a)) {
                    eprintln!("altflip: {e}");
                }
            }
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                let l = handle.autolaunch();
                if settings.launch_at_login && !l.is_enabled().unwrap_or(false) {
                    let _ = l.enable();
                }
            }
            let launched_in_background = std::env::args().any(|a| a == "--background");
            if !settings.onboarded || !pending.is_empty() {
                show_main(&handle, if pending.is_empty() { None } else { Some("activity") });
            } else if !settings.start_minimized && !launched_in_background {
                show_main(&handle, None);
            } else {
                refresh_dock(&handle);
            }
            background::start(handle);
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                let app = window.app_handle();
                let to_tray = app.state::<AppState>().engine.settings().map(|s| s.close_to_tray).unwrap_or(true);
                if to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                    refresh_dock(app);
                } else {
                    app.exit(0);
                }
            }
            ("quick", WindowEvent::Focused(false)) => hide_quick(window.app_handle()),
            ("main" | "quick", WindowEvent::Focused(true)) => background::refresh_soon(window.app_handle()),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_overview,
            commands::plan_switch,
            commands::switch_account,
            commands::save_current,
            commands::start_login,
            commands::cancel_login,
            commands::update_account,
            commands::reorder_accounts,
            commands::remove_account,
            commands::get_history,
            commands::get_activity,
            commands::resolve_incomplete,
            commands::undo_switch,
            commands::refresh_usage,
            commands::claude_status_line,
            commands::import_accounts,
            commands::set_claude_status_line,
            commands::get_settings,
            commands::set_setting,
            commands::reset_settings,
            commands::export_backup,
            commands::import_backup,
            commands::diagnostics,
            commands::app_info,
            commands::show_main_window,
            commands::hide_quick,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building AltFlip");

    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            show_main(app, None);
        }
        let _ = (app, event);
    });
}
