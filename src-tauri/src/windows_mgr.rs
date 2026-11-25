//! Lazy WebView2 window lifecycle management.
//!
//! Deadlock-free WebView2 teardown on Windows 11:
//! - Never call `window.destroy()` synchronously inside `WindowEvent::CloseRequested`
//!   (re-entrant `ICoreWebView2Controller::Close` inside `WM_CLOSE` causes `AppHangXProcB1`).
//! - Allowing normal close lets Tauri cleanly destroy the WebView2 instance while
//!   `RunEvent::ExitRequested { api.prevent_exit() }` keeps the Rust tray daemon alive.

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

use crate::{logger, AppState, PendingCapture};

pub const SETTINGS_LABEL: &str = "settings";
pub const OVERLAY_LABEL: &str = "screenshot-overlay";

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    logger::log_line("INFO", "opening settings window");
    let win = WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
        .title("KeyCraft — Настройки")
        .inner_size(860.0, 600.0)
        .min_inner_size(780.0, 540.0)
        .center()
        .resizable(true)
        .build()
        .map_err(|e| {
            let msg = format!("failed to open settings window: {e}");
            logger::log_line("ERROR", &msg);
            msg
        })?;

    win.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            logger::log_line("INFO", "settings window destroyed (WebView2 unloaded)");
        }
    });

    Ok(())
}

/// Capture screen off the UI thread, then spawn the frameless overlay on the main thread.
pub fn open_screenshot_overlay_async(app: AppHandle, mode: String) {
    logger::log_line("INFO", &format!("screenshot requested (mode={mode})"));
    std::thread::Builder::new()
        .name("keycraft-capture-worker".to_string())
        .spawn(move || {
            let state = app.state::<AppState>();
            let frame = match state.platform.capture_virtual_screen() {
                Ok(f) => f,
                Err(e) => {
                    logger::log_line("ERROR", &format!("screen capture failed: {e}"));
                    return;
                }
            };

            let origin_x = frame.origin_x;
            let origin_y = frame.origin_y;
            let width = frame.width;
            let height = frame.height;

            if let Ok(mut pending) = state.pending_capture.lock() {
                *pending = Some(PendingCapture {
                    frame,
                    mode: mode.clone(),
                });
            };

            let app_ui = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(existing) = app_ui.get_webview_window(OVERLAY_LABEL) {
                    let _ = existing.close();
                }

                let url = format!("overlay.html?mode={mode}");
                match WebviewWindowBuilder::new(
                    &app_ui,
                    OVERLAY_LABEL,
                    WebviewUrl::App(url.into()),
                )
                .title("KeyCraft Скриншот")
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .build()
                {
                    Ok(win) => {
                        let _ = win.set_position(PhysicalPosition::new(origin_x, origin_y));
                        let _ = win.set_size(PhysicalSize::new(width, height));
                        let _ = win.set_focus();

                        let app_handle = app_ui.clone();
                        win.on_window_event(move |event| {
                            if let WindowEvent::Destroyed = event {
                                clear_pending_capture(&app_handle);
                                logger::log_line("INFO", "screenshot overlay destroyed");
                            }
                        });
                    }
                    Err(e) => {
                        clear_pending_capture(&app_ui);
                        logger::log_line(
                            "ERROR",
                            &format!("failed to create screenshot overlay: {e}"),
                        );
                    }
                }
            });
        })
        .ok();
}

pub fn close_screenshot_overlay(app: &AppHandle) {
    clear_pending_capture(app);
    let app_ui = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app_ui.get_webview_window(OVERLAY_LABEL) {
            let _ = win.close();
        }
    });
}

fn clear_pending_capture(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut pending) = state.pending_capture.lock() {
        *pending = None;
    };
}
