use crate::{commands, show_main, toggle_quick, AppState};
use altflip_engine::Overview;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

pub const TRAY_ID: &str = "altflip";

fn icon() -> tauri::Result<tauri::image::Image<'static>> {
    let bytes: &[u8] = if cfg!(target_os = "macos") {
        include_bytes!("../icons/tray@2x.png")
    } else {
        include_bytes!("../icons/tray-32.png")
    };
    tauri::image::Image::from_bytes(bytes).map(|i| i.to_owned())
}

fn build_menu(app: &AppHandle, overview: Option<&Overview>) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if let Some(o) = overview {
        if !o.incomplete.is_empty() || o.providers.iter().any(|p| p.activation_state.as_deref() == Some("drifted")) {
            menu.append(&MenuItem::with_id(app, "attention", "⚠ Attention needed…", true, None::<&str>)?)?;
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
        let mut any = false;
        for p in o.providers.iter().filter(|p| p.info.installed || !p.accounts.is_empty()) {
            any = true;
            let title =
                if p.info.experimental { format!("{} (experimental)", p.info.name) } else { p.info.name.clone() };
            menu.append(&MenuItem::with_id(app, format!("hdr:{}", p.info.id.as_str()), title, false, None::<&str>)?)?;
            if p.accounts.is_empty() {
                menu.append(&MenuItem::with_id(
                    app,
                    format!("add:{}", p.info.id.as_str()),
                    "    Add an account…",
                    true,
                    None::<&str>,
                )?)?;
            }
            for a in &p.accounts {
                let mut label = format!("    {}", a.row.label);
                if let Some(e) = &a.row.email {
                    if !a.row.label.contains(e.as_str()) {
                        label.push_str(&format!("  ·  {e}"));
                    }
                }
                let active = p.active_account_id.as_deref() == Some(a.row.id.as_str());
                let item = CheckMenuItem::with_id(
                    app,
                    format!("acct:{}", a.row.id),
                    label,
                    p.switching_enabled || active,
                    active,
                    None::<&str>,
                )?;
                menu.append(&item)?;
            }
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
        if !any {
            menu.append(&MenuItem::with_id(app, "none", "No supported tools detected", false, None::<&str>)?)?;
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
    }
    menu.append(&MenuItem::with_id(app, "quick", "Quick Switcher", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "open", "Manage Accounts…", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit AltFlip", true, None::<&str>)?)?;
    Ok(menu)
}

fn tooltip(o: &Overview) -> String {
    let parts: Vec<String> = o
        .providers
        .iter()
        .filter_map(|p| {
            let id = p.active_account_id.as_deref()?;
            let a = p.accounts.iter().find(|a| a.row.id == id)?;
            Some(format!("{}: {}", p.info.name, a.row.label))
        })
        .collect();
    if parts.is_empty() {
        "AltFlip".into()
    } else {
        format!("AltFlip\n{}", parts.join("\n"))
    }
}

fn on_account(app: &AppHandle, id: String) {
    let app = app.clone();
    std::thread::spawn(move || {
        let engine = app.state::<AppState>().engine.clone();
        let plan = match engine.plan_switch(&id) {
            Ok(p) => p,
            Err(e) => {
                crate::notify(&app, "Cannot switch", &e.to_string(), true);
                return;
            }
        };
        if plan.already_active {
            commands::broadcast(&app);
            return;
        }
        if let Some(reason) = plan.blocked {
            crate::notify(&app, "Cannot switch", &reason, true);
            commands::broadcast(&app);
            return;
        }
        if plan.requires_confirmation {
            show_main(&app, None);
            let _ = app.emit_to("main", "altflip://confirm-switch", &id);
            return;
        }
        let _ = commands::run_switch(&app, &id, false, "tray");
    });
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let overview = app.state::<AppState>().engine.overview(false).ok();
    let menu = build_menu(app, overview.as_ref())?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon()?)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip(overview.as_ref().map(tooltip).unwrap_or_else(|| "AltFlip".into()))
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "linux"))
        .on_menu_event(|app, e| {
            let id = e.id.as_ref();
            match id {
                "quick" => toggle_quick(app, None),
                "open" | "attention" => show_main(app, if id == "attention" { Some("activity") } else { None }),
                "settings" => show_main(app, Some("settings")),
                "quit" => app.exit(0),
                _ if id.starts_with("acct:") => on_account(app, id[5..].to_string()),
                _ if id.starts_with("add:") => {
                    show_main(app, Some("accounts"));
                    let _ = app.emit_to("main", "altflip://add-account", &id[4..]);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left, button_state: MouseButtonState::Up, rect, ..
            } = event
            {
                let app = tray.app_handle();
                let dashboard =
                    app.state::<AppState>().engine.settings().map(|s| s.tray_click == "dashboard").unwrap_or(false);
                if dashboard {
                    show_main(app, None);
                } else {
                    toggle_quick(app, Some(rect));
                }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn refresh(app: &AppHandle, o: &Overview) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    if let Ok(menu) = build_menu(app, Some(o)) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(tooltip(o)));
}

pub fn set_busy(app: &AppHandle, busy: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if busy {
            let _ = tray.set_tooltip(Some("AltFlip — switching…"));
        } else if let Ok(o) = app.state::<AppState>().engine.overview(false) {
            let _ = tray.set_tooltip(Some(tooltip(&o)));
        }
    }
}
