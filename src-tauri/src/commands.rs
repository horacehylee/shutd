use chrono::{DateTime, Local};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartExt;

use crate::config::Config;
use crate::{config, scheduler, toast, AppState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToastPayload {
    pub dismiss_at: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    pub start_time: String,
    pub snooze_interval: u32,
    pub before: u32,
    pub duration: u32,
    pub autostart: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatePayload {
    pub shutdown_at: DateTime<Local>,
    pub snoozed: u32,
    pub toast: Option<ToastPayload>,
    pub settings: SettingsPayload,
}

pub fn build_payload(app: &AppHandle) -> StatePayload {
    let state = app.state::<AppState>();
    let config = state.config.lock().expect("config lock").clone();
    let schedule = *state.schedule.lock().expect("schedule lock");
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    StatePayload {
        shutdown_at: schedule.shutdown_at,
        snoozed: schedule.snoozed,
        toast: schedule.toast.map(|t| ToastPayload { dismiss_at: t.dismiss_at }),
        settings: SettingsPayload {
            start_time: config.start_time,
            snooze_interval: config.snooze_interval,
            before: config.notification.before,
            duration: config.notification.duration,
            autostart,
        },
    }
}

/// Updates the tray label and pushes fresh state to the frontend.
pub fn emit_state(app: &AppHandle) -> StatePayload {
    let payload = build_payload(app);
    if let Some(menu) = app.try_state::<crate::TrayMenu>() {
        let _ = menu.next.set_text(format!("Shutdown at {}", payload.shutdown_at.format("%H:%M")));
    }
    let _ = app.emit("state-changed", &payload);
    payload
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> StatePayload {
    build_payload(&app)
}

#[tauri::command]
pub fn snooze(app: AppHandle) -> StatePayload {
    let state = app.state::<AppState>();
    {
        let config = state.config.lock().expect("config lock");
        let mut schedule = state.schedule.lock().expect("schedule lock");
        schedule.snooze(&config);
    }
    log::info!("snoozed, shutdown at {}", build_payload(&app).shutdown_at.format("%H:%M"));
    toast::hide(&app);
    emit_state(&app)
}

#[tauri::command]
pub fn dismiss_toast(app: AppHandle) -> StatePayload {
    {
        let state = app.state::<AppState>();
        let mut schedule = state.schedule.lock().expect("schedule lock");
        // Dismissal is sticky: the scheduler must not re-arm this toast for
        // the same shutdown time.
        schedule.dismissed_for = Some(schedule.shutdown_at);
        schedule.toast = None;
    }
    toast::hide(&app);
    emit_state(&app)
}

/// Saves the settings to disk and hot-reloads the scheduler, replacing the
/// old watch-the-config-file round trip from the frontend.
#[tauri::command]
pub fn set_settings(app: AppHandle, config: Config) -> Result<StatePayload, String> {
    scheduler::parse_time(&config.start_time)?;
    config::save(&config)?;
    {
        let state = app.state::<AppState>();
        *state.config.lock().expect("config lock") = config.clone();
        state
            .schedule
            .lock()
            .expect("schedule lock")
            .reconfigure(&config)?;
    }
    log::info!("config updated: start {}", config.start_time);
    Ok(emit_state(&app))
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    }
    .map_err(|e| e.to_string())
}
