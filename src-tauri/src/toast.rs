use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};

use crate::{commands, panel, webview_memory};

pub const WINDOW: &str = "toast";
pub const WIDTH: f64 = 340.0;
pub const HEIGHT: f64 = 132.0;

pub fn window<'a>(app: &'a AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(WINDOW)
}

/// Shows the snooze toast above the taskbar. The window is non-focusable, so
/// it never steals focus from the user's active app.
pub fn show(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    webview_memory::set_inactive(&w, false);
    let _ = w.set_size(LogicalSize::new(WIDTH, HEIGHT));
    panel::anchor_bottom_right(&w, WIDTH, HEIGHT);
    let _ = w.show();
    commands::emit_state(app);
}

pub fn hide(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    if w.hide().is_err() {
        return;
    }
    webview_memory::set_inactive(&w, true);
}
