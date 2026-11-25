//! Lazy WebView2 window lifecycle management.
//!
//! RAM budget guarantee (`<= 15-20 MB` idle):
//! - App launches with zero WebView windows.
//! - Settings and screenshot overlay windows are created on demand.
//! - Closing any WebView window invokes `window.destroy()` so WebView2 processes unload completely.

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::{AppState, PendingCapture};

pub const SETTINGS_LABEL: &str = "settings";
pub const OVERLAY_LABEL: &str = "screenshot-overlay";

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    let win = WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("KeyCraft — Настройки")
    .inner_size(860.0, 600.0)
    .min_inner_size(780.0, 540.0)
    .center()
    .resizable(true)
    .build()
    .map_err(|e| format!("failed to open settings window: {e}"))?;

    // Destroy WebView2 completely on close to reclaim RAM immediately.
    let win_clone = win.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_clone.destroy();
        }
    });

    Ok(())
}

pub fn open_screenshot_overlay(app: &AppHandle, mode: &str) -> Result<(), String> {
    // If overlay is already open, close it first
    if let Some(existing) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = existing.destroy();
    }

    let state = app.state::<AppState>();
    let frame = state
        .platform
        .capture_virtual_screen()
        .map_err(|e| format!("capture failed: {e}"))?;

    let origin_x = frame.origin_x;
    let origin_y = frame.origin_y;
    let width = frame.width;
    let height = frame.height;

    if let Ok(mut pending) = state.pending_capture.lock() {
        *pending = Some(PendingCapture {
            frame,
            mode: mode.to_string(),
        });
    }

    let url = format!("overlay.html?mode={mode}");
    let win = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App(url.into()))
        .title("KeyCraft Скриншот")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .build()
        .map_err(|e| format!("failed to create screenshot overlay: {e}"))?;

    let _ = win.set_position(PhysicalPosition::new(origin_x, origin_y));
    let _ = win.set_size(PhysicalSize::new(width, height));
    let _ = win.set_focus();

    let app_handle = app.clone();
    let win_clone = win.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            clear_pending_capture(&app_handle);
            let _ = win_clone.destroy();
        }
    });

    Ok(())
}

pub fn close_screenshot_overlay(app: &AppHandle) {
    clear_pending_capture(app);
    if let Some(win) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = win.destroy();
    }
}

fn clear_pending_capture(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut pending) = state.pending_capture.lock() {
        *pending = None;
    };
}
