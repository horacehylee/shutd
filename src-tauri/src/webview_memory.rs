use tauri::WebviewWindow;

/// Suspends the WebView2 renderer while the window is hidden so idle memory
/// drops to scheduler-only levels; mirrored from the OpenQuota approach.
#[cfg(target_os = "windows")]
pub fn set_inactive(window: &WebviewWindow, inactive: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows::core::Interface;

    let target = if inactive {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
    } else {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
    };

    let _ = window.with_webview(move |webview| {
        let result = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|core| core.cast::<ICoreWebView2_19>())
                .and_then(|memory| memory.SetMemoryUsageTargetLevel(target))
        };
        if result.is_err() {
            log::debug!("webview memory target is unavailable");
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn set_inactive(_window: &WebviewWindow, _inactive: bool) {}
