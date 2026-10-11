mod commands;
mod config;
mod logging;
mod panel;
mod scheduler;
mod shutdown;
mod tray;
mod webview_memory;

use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::Duration;

use chrono::Local;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

use crate::config::Config;
use crate::panel::DismissGuard;
use crate::scheduler::Schedule;

/// Shared scheduler state behind the Tauri-managed mutexes.
pub struct AppState {
    pub config: Mutex<Config>,
    pub schedule: Mutex<Schedule>,
}

/// Handle to the tray menu so the shutdown-time label stays current.
pub struct TrayMenu {
    pub next: MenuItem<Wry>,
}

const TRAY_ID: &str = "shutd-tray";

pub fn run() {
    if let Err(e) = std::fs::create_dir_all(config::dir()) {
        eprintln!("failed to create config dir: {e}");
    }
    if let Err(e) = logging::init(&config::log_path()) {
        eprintln!("failed to init logging: {e}");
    }
    log::info!("started");

    let config = config::load_or_create().unwrap_or_else(|e| {
        log::error!("{e}; using defaults");
        Config::default()
    });
    let schedule = Schedule::new(&config).unwrap_or_else(|e| {
        log::error!("{e}; using 01:00");
        Schedule::new(&Config::default()).expect("default config is valid")
    });
    log::info!("shutdown at {}", schedule.shutdown_at.format("%H:%M"));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            panel::show_panel(app);
        }))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            config: Mutex::new(config),
            schedule: Mutex::new(schedule),
        })
        .manage(DismissGuard::default())
        .manage(Mutex::new(panel::Mode::Panel))
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::snooze,
            commands::dismiss_toast,
            commands::set_settings,
            commands::set_autostart,
        ])
        .setup(|app| {
            install_tray(app.handle())?;
            let handle = app.handle().clone();
            thread::spawn(move || scheduler_loop(handle));
            let handle = app.handle().clone();
            thread::spawn(move || watch_config(handle));
            let handle = app.handle().clone();
            thread::spawn(move || theme_loop(handle));
            Ok(())
        })
        .on_window_event(|window, event| panel::handle_window_event(window, event))
        .run(tauri::generate_context!())
        .expect("error while running shutd");
}

fn install_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let next = MenuItem::with_id(app, "next", "Shutdown at ?", false, None::<&str>)?;
    let snooze = MenuItem::with_id(app, "snooze", "Snooze", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&next, &separator, &snooze, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray::render_glyph(tray::taskbar_uses_light_theme()))
        .menu(&menu)
        .tooltip("Shutd")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "snooze" => {
                commands::snooze(app.clone());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Feeds the positioner the tray's live position so TrayCenter works.
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                panel::toggle(tray.app_handle());
            }
        })
        .build(app)?;

    app.manage(TrayMenu { next });
    Ok(())
}

/// Polls the schedule twice a second; cheap, and immune to clock changes
/// that would strand a long sleep.
fn scheduler_loop(app: AppHandle) {
    loop {
        thread::sleep(Duration::from_millis(500));
        let now = Local::now();
        let state = app.state::<AppState>();
        let action = {
            let config = state.config.lock().expect("config lock");
            let mut schedule = state.schedule.lock().expect("schedule lock");
            if now >= schedule.shutdown_at {
                Some("shutdown")
            } else if schedule.toast_due(&config, now).is_some() {
                Some("toast")
            } else if schedule.toast_expired(now) {
                Some("toast-expired")
            } else {
                None
            }
        };
        drop(state);
        match action {
            Some("shutdown") => {
                log::info!("initiating shutdown");
                if let Err(e) = shutdown::exec() {
                    log::error!("{e}");
                }
                app.exit(0);
                return;
            }
            Some("toast") => {
                panel::show_toast(&app);
                commands::emit_state(&app);
            }
            Some("toast-expired") => {
                log::info!("toast expired without snooze");
                panel::hide(&app);
                commands::emit_state(&app);
            }
            _ => {}
        }
    }
}

/// Hot-reloads config.yaml; the settings screen and manual edits both land here.
fn watch_config(app: AppHandle) {
    let (tx, rx) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(w) => w,
        Err(e) => {
            log::error!("config watcher unavailable: {e}");
            return;
        }
    };
    if let Err(e) = notify::Watcher::watch(&mut watcher, &config::dir(), notify::RecursiveMode::NonRecursive) {
        log::error!("config watch failed: {e}");
        return;
    }
    while rx.recv().is_ok() {
        // Debounce: editors often fire several events per save.
        while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
        match std::fs::read_to_string(config::config_path())
            .map_err(|e| e.to_string())
            .and_then(|yaml| config::parse(&yaml))
        {
            Ok(new_config) => {
                let state = app.state::<AppState>();
                let mut schedule = state.schedule.lock().expect("schedule lock");
                let result = schedule.reconfigure(&new_config);
                drop(schedule);
                if result.is_ok() {
                    *state.config.lock().expect("config lock") = new_config;
                    log::info!("config reloaded");
                    commands::emit_state(&app);
                } else {
                    log::error!("reloaded config has invalid time; keeping previous");
                }
            }
            Err(e) => log::error!("ignoring bad config change: {e}"),
        }
    }
}

/// Re-renders the tray glyph when the taskbar theme flips.
fn theme_loop(app: AppHandle) {
    let mut light = tray::taskbar_uses_light_theme();
    loop {
        thread::sleep(Duration::from_secs(60 * 60));
        let current = tray::taskbar_uses_light_theme();
        if current != light {
            light = current;
            if let Some(icon_tray) = app.tray_by_id(TRAY_ID) {
                let _ = icon_tray.set_icon(Some(tray::render_glyph(current)));
            }
        }
    }
}
