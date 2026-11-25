//! KeyCraft Tauri v2 host library: background daemon, tray, offline capture URI protocol,
//! and lazy WebView2 window lifecycle.

mod commands;
mod daemon;
mod logger;
mod store;
mod tray;
mod windows_mgr;

use std::sync::{Arc, Mutex};

use keycraft_core_engine::KeyCraftEngine;
use keycraft_platform::{create_platform_backend, CapturedFrame, PlatformBackend};
use tauri::{AppHandle, Manager, RunEvent};

pub struct PendingCapture {
    pub frame: CapturedFrame,
    pub mode: String,
}

pub struct AppState {
    pub engine: Arc<Mutex<KeyCraftEngine>>,
    pub sandbox_engine: Arc<Mutex<KeyCraftEngine>>,
    pub platform: Arc<dyn PlatformBackend>,
    pub pending_capture: Arc<Mutex<Option<PendingCapture>>>,
}

pub fn toggle_enabled_from_backend(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (new_enabled, cfg) = {
        let Ok(mut eng) = state.engine.lock() else {
            return;
        };
        let mut cfg = eng.config().clone();
        cfg.enabled = !cfg.enabled;
        let flag = cfg.enabled;
        eng.update_config(cfg.clone());
        (flag, cfg)
    };
    let _ = store::save_config(&cfg);
    let layout = state.platform.current_layout();
    tray::refresh_tray_visuals(app, layout, new_enabled, cfg.screenshot.enabled);
}

pub fn sync_platform_screenshot_hotkeys(
    platform: &dyn PlatformBackend,
    cfg: &keycraft_core_engine::KeyCraftConfig,
) {
    let warns = platform.sync_screenshot_hotkeys(
        cfg.screenshot.enabled,
        &cfg.hotkeys.screenshot_region,
        &cfg.hotkeys.screenshot_window,
        &cfg.hotkeys.screenshot_fullscreen,
    );
    if !cfg.screenshot.enabled {
        logger::log_line("INFO", "screenshot hotkeys unregistered (disabled)");
    } else if warns.is_empty() {
        logger::log_line(
            "INFO",
            &format!(
                "RegisterHotKey OK: region={}, window={}, fullscreen={}",
                cfg.hotkeys.screenshot_region,
                cfg.hotkeys.screenshot_window,
                cfg.hotkeys.screenshot_fullscreen
            ),
        );
    } else {
        logger::log_line(
            "WARN",
            &format!("RegisterHotKey warnings: {}", warns.join("; ")),
        );
    }
}

pub fn run() {
    logger::install_panic_hook();
    logger::log_line("INFO", concat!("KeyCraft v", env!("CARGO_PKG_VERSION"), " starting"));

    let mut initial_cfg = store::load_config();
    let platform_box = create_platform_backend().expect("initialize platform backend");
    initial_cfg.system.autostart = platform_box.is_autostart_enabled();

    let platform: Arc<dyn PlatformBackend> = Arc::from(platform_box);
    let initial_layout = platform.current_layout();
    let initial_enabled = initial_cfg.enabled;
    let initial_screenshot_enabled = initial_cfg.screenshot.enabled;

    let engine = Arc::new(Mutex::new(KeyCraftEngine::new(initial_cfg.clone())));
    let sandbox_engine = Arc::new(Mutex::new(KeyCraftEngine::new(initial_cfg)));
    let pending_capture: Arc<Mutex<Option<PendingCapture>>> = Arc::new(Mutex::new(None));

    let capture_for_proto = Arc::clone(&pending_capture);

    let app = tauri::Builder::default()
        .manage(AppState {
            engine,
            sandbox_engine,
            platform: Arc::clone(&platform),
            pending_capture,
        })
        // Zero-disk, zero-network custom URI protocol streaming the in-memory screenshot PNG to the overlay
        .register_uri_scheme_protocol("keycraft-capture", move |_ctx, _req| {
            let bytes = capture_for_proto
                .lock()
                .ok()
                .and_then(|g| g.as_ref().map(|c| c.frame.png_bytes.clone()))
                .unwrap_or_default();

            tauri::http::Response::builder()
                .status(if bytes.is_empty() { 404 } else { 200 })
                .header("Content-Type", "image/png")
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store")
                .body(bytes)
                .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::reset_config,
            commands::export_config_json,
            commands::import_config_json,
            commands::toggle_enabled,
            commands::get_runtime_status,
            commands::add_dictionary_word,
            commands::list_running_apps,
            commands::sandbox_process_event,
            commands::sandbox_reset,
            commands::trigger_screenshot,
            commands::get_capture_metadata,
            commands::get_capture_image_data_url,
            commands::screenshot_copy_png,
            commands::screenshot_save_png,
            commands::screenshot_pin_png,
            commands::screenshot_close,
        ])
        .setup(move |app| {
            tray::setup_tray(
                app.handle(),
                initial_layout,
                initial_enabled,
                initial_screenshot_enabled,
            )?;
            daemon::start_daemon(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building KeyCraft");

    // Keep the pure Rust daemon alive in the tray when all WebView2 windows are destroyed!
    let platform_exit = Arc::clone(&platform);
    app.run(move |_app_handle, event| match event {
        RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => {
            platform_exit.stop_hooks();
        }
        _ => {}
    });
}
