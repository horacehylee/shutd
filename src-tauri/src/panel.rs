use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::webview_memory;

pub const WINDOW: &str = "main";
pub const PANEL_WIDTH: f64 = 320.0;
pub const PANEL_HEIGHT: f64 = 272.0;
pub const TOAST_WIDTH: f64 = 340.0;
pub const TOAST_HEIGHT: f64 = 132.0;
const TOAST_MARGIN: f64 = 16.0;
const TOAST_BOTTOM_CLEARANCE: f64 = 56.0; // above the taskbar
const BLUR_DISMISS_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Panel,
    Toast,
}

/// Cancels stale blur dismissals when a tray click races a focus-loss event.
/// A new token invalidates every pending dismissal captured before it.
#[derive(Default)]
pub struct DismissGuard {
    generation: AtomicU64,
}

impl DismissGuard {
    pub fn cancel_pending(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn token(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn is_current(&self, token: u64) -> bool {
        self.token() == token
    }
}

pub fn window<'a>(app: &'a AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(WINDOW)
}

pub fn current_mode(app: &AppHandle) -> Mode {
    app.state::<Mutex<Mode>>()
        .lock()
        .map(|mode| *mode)
        .unwrap_or(Mode::Panel)
}

/// Opens the status/settings panel anchored under the tray icon.
pub fn show_panel(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    app.state::<DismissGuard>().cancel_pending();
    webview_memory::set_inactive(&w, false);
    set_mode(app, Mode::Panel);
    let _ = w.set_size(LogicalSize::new(PANEL_WIDTH, PANEL_HEIGHT));
    if let Err(e) = w.as_ref().window().move_window_constrained(Position::TrayCenter) {
        log::warn!("tray positioning failed: {e}; anchoring to bottom-right");
        anchor_bottom_right(&w, PANEL_WIDTH, PANEL_HEIGHT);
    }
    let _ = w.show();
    let _ = w.set_focus();
}

/// Opens the snooze toast above the bottom-right corner of the primary monitor.
pub fn show_toast(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    app.state::<DismissGuard>().cancel_pending();
    webview_memory::set_inactive(&w, false);
    set_mode(app, Mode::Toast);
    let _ = w.set_size(LogicalSize::new(TOAST_WIDTH, TOAST_HEIGHT));
    anchor_bottom_right(&w, TOAST_WIDTH, TOAST_HEIGHT);
    let _ = w.show();
    let _ = w.set_focus();
}

pub fn hide(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    if w.hide().is_err() {
        return;
    }
    webview_memory::set_inactive(&w, true);
}

/// Positions a window above the bottom-right corner of the primary monitor,
/// clearing the taskbar. Fallback when the tray anchor is unavailable.
fn anchor_bottom_right(window: &WebviewWindow, width: f64, height: f64) {
    let Some(monitor) = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.app_handle().primary_monitor().ok().flatten())
    else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let x = f64::from(monitor.size().width) / scale - width - TOAST_MARGIN;
    let y = f64::from(monitor.size().height) / scale
        - height
        - TOAST_MARGIN
        - TOAST_BOTTOM_CLEARANCE;
    let _ = window.set_position(LogicalPosition::new(x.max(0.0), y.max(0.0)));
}

pub fn toggle(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    let visible = w.is_visible().unwrap_or(false);
    if visible {
        hide(app);
    } else {
        show_panel(app);
    }
}

fn set_mode(app: &AppHandle, mode: Mode) {
    if let Ok(mut current) = app.state::<Mutex<Mode>>().lock() {
        *current = mode;
    }
}

/// Hides the window shortly after losing focus unless a newer user action
/// (another tray click) invalidated the dismissal.
fn schedule_blur_dismiss(app: AppHandle, token: u64) {
    thread::spawn(move || {
        thread::sleep(BLUR_DISMISS_DELAY);
        let app_for_dismiss = app.clone();
        let _ = app.run_on_main_thread(move || {
            let guard_current = app_for_dismiss.state::<DismissGuard>().is_current(token);
            let Some(w) = window(&app_for_dismiss) else { return };
            let still_unfocused = w.is_focused().is_ok_and(|focused| !focused);
            if guard_current && still_unfocused {
                hide(&app_for_dismiss);
                app_for_dismiss.emit("panel-hidden", ()).ok();
            }
        });
    });
}

pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != WINDOW {
        return;
    }
    match event {
        WindowEvent::Focused(false) => {
            let app = window.app_handle().clone();
            let token = app.state::<DismissGuard>().token();
            schedule_blur_dismiss(app, token);
        }
        WindowEvent::CloseRequested { api, .. } => {
            // The tray Quit item is the real exit; closing hides the panel.
            api.prevent_close();
            hide(window.app_handle());
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::DismissGuard;

    #[test]
    fn tray_click_invalidates_pending_blur_dismissal() {
        let guard = DismissGuard::default();
        let pending = guard.token();
        guard.cancel_pending();
        assert!(!guard.is_current(pending));
    }

    #[test]
    fn unchanged_token_stays_current() {
        let guard = DismissGuard::default();
        let pending = guard.token();
        assert!(guard.is_current(pending));
    }
}
