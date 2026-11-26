//! Tauri v2 IPC commands connecting the Svelte 5 Settings & Screenshotter UI to the Rust daemon.

use std::fs;
use std::path::PathBuf;

use base64::Engine as _;
use chrono::Local;
use keycraft_core_engine::{
    ActiveAppState, EngineAction, EngineInput, KeyCraftConfig, Language, WordSource,
};
use keycraft_platform::{RunningAppInfo, ScreenRect};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::{store, tray, windows_mgr, AppState};

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStatus {
    pub enabled: bool,
    pub layout: Language,
    pub ru_dict_size: usize,
    pub en_dict_size: usize,
    pub autostart: bool,
    pub os_snipping_prtsc_enabled: bool,
    pub version: String,
    pub config_path: String,
    pub default_screenshots_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureMetadata {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
    pub mode: String,
    pub active_window_rect: Option<ScreenRect>,
    pub visible_windows: Vec<ScreenRect>,
    pub show_magnifier: bool,
    pub default_color: String,
    pub default_stroke_width: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SandboxInputDto {
    Char { ch: String, layout: Language },
    Space { layout: Language },
    Tab { layout: Language },
    Enter { layout: Language },
    Backspace,
    Reset,
    Undo,
    SwapSymbol { layout: Language },
    ManualConvert,
    CycleCase,
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<KeyCraftConfig, String> {
    let eng = state.engine.lock().map_err(|_| "engine lock poisoned")?;
    Ok(eng.config().clone())
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: KeyCraftConfig,
) -> Result<KeyCraftConfig, String> {
    let autostart = config.system.autostart;
    let enabled = config.enabled;

    store::save_config(&config)?;
    let _ = state.platform.set_autostart(autostart);
    crate::sync_platform_screenshot_hotkeys(state.platform.as_ref(), &config);

    {
        let mut eng = state.engine.lock().map_err(|_| "engine lock poisoned")?;
        eng.update_config(config.clone());
    }
    if let Ok(mut sb) = state.sandbox_engine.lock() {
        sb.update_config(config.clone());
    }

    let layout = state.platform.current_layout();
    tray::refresh_tray_visuals(&app, layout, enabled, config.screenshot.enabled);
    Ok(config)
}

#[tauri::command]
pub fn reset_config(app: AppHandle, state: State<'_, AppState>) -> Result<KeyCraftConfig, String> {
    let fresh = KeyCraftConfig::default();
    save_config(app, state, fresh)
}

#[tauri::command]
pub fn export_config_json(state: State<'_, AppState>) -> Result<String, String> {
    let eng = state.engine.lock().map_err(|_| "engine lock poisoned")?;
    serde_json::to_string_pretty(eng.config()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_config_json(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
) -> Result<KeyCraftConfig, String> {
    let parsed: KeyCraftConfig =
        serde_json::from_str(&json).map_err(|e| format!("Некорректный JSON настроек: {e}"))?;
    save_config(app, state, parsed)
}

#[tauri::command]
pub fn toggle_enabled(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let (new_enabled, cfg) = {
        let mut eng = state.engine.lock().map_err(|_| "engine lock poisoned")?;
        let mut cfg = eng.config().clone();
        cfg.enabled = !cfg.enabled;
        let flag = cfg.enabled;
        eng.update_config(cfg.clone());
        (flag, cfg)
    };
    let _ = store::save_config(&cfg);
    let layout = state.platform.current_layout();
    tray::refresh_tray_visuals(&app, layout, new_enabled, cfg.screenshot.enabled);
    Ok(new_enabled)
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let eng = state.engine.lock().map_err(|_| "engine lock poisoned")?;
    Ok(RuntimeStatus {
        enabled: eng.config().enabled,
        layout: state.platform.current_layout(),
        ru_dict_size: eng.dictionary().ru_word_count(),
        en_dict_size: eng.dictionary().en_word_count(),
        autostart: state.platform.is_autostart_enabled(),
        os_snipping_prtsc_enabled: state.platform.is_os_snipping_prtsc_enabled(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        config_path: store::config_path().display().to_string(),
        default_screenshots_dir: store::default_screenshots_dir().display().to_string(),
    })
}

#[tauri::command]
pub fn set_os_snipping_prtsc_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    state
        .platform
        .set_os_snipping_prtsc_enabled(enabled)
        .map_err(|e| e.to_string())?;
    crate::logger::log_line(
        "INFO",
        &format!("Windows PrintScreenKeyForSnippingEnabled set to {enabled}"),
    );
    if let Ok(eng) = state.engine.lock() {
        crate::sync_platform_screenshot_hotkeys(state.platform.as_ref(), eng.config());
    }
    Ok(state.platform.is_os_snipping_prtsc_enabled())
}

#[tauri::command]
pub fn add_dictionary_word(
    state: State<'_, AppState>,
    word: String,
) -> Result<KeyCraftConfig, String> {
    let cfg = {
        let mut eng = state.engine.lock().map_err(|_| "engine lock poisoned")?;
        eng.add_user_word(&word, WordSource::Manual);
        eng.config().clone()
    };
    store::save_config(&cfg)?;
    if let Ok(mut sb) = state.sandbox_engine.lock() {
        sb.update_config(cfg.clone());
    }
    Ok(cfg)
}

#[tauri::command]
pub fn list_running_apps(state: State<'_, AppState>) -> Vec<RunningAppInfo> {
    state.platform.list_running_apps()
}

#[tauri::command]
pub fn sandbox_process_event(
    state: State<'_, AppState>,
    event: SandboxInputDto,
) -> Result<EngineAction, String> {
    let mut sb = state
        .sandbox_engine
        .lock()
        .map_err(|_| "sandbox lock poisoned")?;
    let app_ctx = ActiveAppState {
        process_name: "keycraft-sandbox".to_string(),
        is_password_field: false,
    };

    let input = match event {
        SandboxInputDto::Char { ch, layout } => {
            let Some(c) = ch.chars().next() else {
                return Ok(EngineAction::NoOp);
            };
            EngineInput::Char { ch: c, layout }
        }
        SandboxInputDto::Space { layout } => EngineInput::Space { layout },
        SandboxInputDto::Tab { layout } => EngineInput::Tab { layout },
        SandboxInputDto::Enter { layout } => EngineInput::Enter { layout },
        SandboxInputDto::Backspace => EngineInput::Backspace,
        SandboxInputDto::Reset => EngineInput::ResetContext,
        SandboxInputDto::Undo => EngineInput::UndoLast,
        SandboxInputDto::SwapSymbol { layout } => EngineInput::SwapLastSymbol { layout },
        SandboxInputDto::ManualConvert => EngineInput::ManualConvertLastWord,
        SandboxInputDto::CycleCase => EngineInput::ManualCycleCaseLastWord,
    };

    Ok(sb.process(input, &app_ctx))
}

#[tauri::command]
pub fn sandbox_reset(state: State<'_, AppState>) -> Result<(), String> {
    let mut sb = state
        .sandbox_engine
        .lock()
        .map_err(|_| "sandbox lock poisoned")?;
    sb.reset_buffer();
    Ok(())
}

#[tauri::command]
pub async fn trigger_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    let enabled = state
        .engine
        .lock()
        .map(|e| e.config().screenshot.enabled)
        .unwrap_or(true);
    if !enabled {
        return Ok(());
    }
    windows_mgr::open_screenshot_overlay_async(app, mode);
    Ok(())
}

#[tauri::command]
pub fn get_capture_image_data_url(state: State<'_, AppState>) -> Result<String, String> {
    let pending = state
        .pending_capture
        .lock()
        .map_err(|_| "capture lock poisoned")?;
    let Some(cap) = pending.as_ref() else {
        return Err("no active capture frame".to_string());
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&cap.frame.png_bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

#[tauri::command]
pub fn get_capture_metadata(state: State<'_, AppState>) -> Result<CaptureMetadata, String> {
    let pending = state
        .pending_capture
        .lock()
        .map_err(|_| "capture lock poisoned")?;
    let Some(cap) = pending.as_ref() else {
        return Err("no active capture frame".to_string());
    };
    let shot_cfg = state
        .engine
        .lock()
        .map(|e| e.config().screenshot.clone())
        .unwrap_or_default();

    Ok(CaptureMetadata {
        origin_x: cap.frame.origin_x,
        origin_y: cap.frame.origin_y,
        width: cap.frame.width,
        height: cap.frame.height,
        mode: cap.mode.clone(),
        active_window_rect: cap.frame.active_window_rect,
        visible_windows: cap.frame.visible_windows.clone(),
        show_magnifier: shot_cfg.show_magnifier,
        default_color: shot_cfg.default_color,
        default_stroke_width: shot_cfg.default_stroke_width,
    })
}

fn decode_png_data_url(png_base64: &str) -> Result<Vec<u8>, String> {
    let clean = png_base64
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(png_base64);
    base64::engine::general_purpose::STANDARD
        .decode(clean)
        .map_err(|e| format!("invalid base64 PNG: {e}"))
}

#[tauri::command]
pub fn screenshot_copy_png(
    app: AppHandle,
    state: State<'_, AppState>,
    png_base64: String,
) -> Result<(), String> {
    let bytes = decode_png_data_url(&png_base64)?;
    state
        .platform
        .copy_png_to_clipboard(&bytes)
        .map_err(|e| e.to_string())?;
    windows_mgr::close_screenshot_overlay(&app);
    Ok(())
}

#[tauri::command]
pub fn screenshot_save_png(
    app: AppHandle,
    state: State<'_, AppState>,
    png_base64: String,
) -> Result<String, String> {
    let bytes = decode_png_data_url(&png_base64)?;
    let shot_cfg = state
        .engine
        .lock()
        .map(|e| e.config().screenshot.clone())
        .unwrap_or_default();

    let dir = if shot_cfg.save_dir.trim().is_empty() {
        store::default_screenshots_dir()
    } else {
        PathBuf::from(shot_cfg.save_dir.trim())
    };
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create screenshot dir: {e}"))?;

    let pattern = if shot_cfg.filename_pattern.trim().is_empty() {
        "KeyCraft_%Y-%m-%d_%H-%M-%S"
    } else {
        shot_cfg.filename_pattern.trim()
    };
    let stem = Local::now().format(pattern).to_string();
    let path = dir.join(format!("{stem}.png"));
    fs::write(&path, &bytes).map_err(|e| format!("failed to save PNG: {e}"))?;

    if shot_cfg.copy_after_save {
        let _ = state.platform.copy_png_to_clipboard(&bytes);
    }

    windows_mgr::close_screenshot_overlay(&app);
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn screenshot_pin_png(
    app: AppHandle,
    state: State<'_, AppState>,
    png_base64: String,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let bytes = decode_png_data_url(&png_base64)?;
    windows_mgr::close_screenshot_overlay(&app);
    state
        .platform
        .pin_screenshot(bytes, screen_x, screen_y)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn screenshot_close(app: AppHandle) {
    windows_mgr::close_screenshot_overlay(&app);
}
