//! Background Rust daemon loop connecting `PlatformBackend` events to `KeyCraftEngine`.

use std::sync::Arc;

use keycraft_core_engine::{
    ActiveAppState, EngineAction, EngineInput, HotkeyConfig, Language, PinnedLayout, WordSource,
};
use keycraft_platform::PlatformEvent;
use tauri::{AppHandle, Manager};

use crate::{store, tray, windows_mgr, AppState};

// Windows virtual key codes
const VK_BACK: u32 = 0x08;
const VK_TAB: u32 = 0x09;
const VK_RETURN: u32 = 0x0D;
const VK_PAUSE: u32 = 0x13;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_SNAPSHOT: u32 = 0x2C; // PrintScreen
const VK_LEFT: u32 = 0x25;
const VK_UP: u32 = 0x26;
const VK_RIGHT: u32 = 0x27;
const VK_DOWN: u32 = 0x28;
const VK_HOME: u32 = 0x24;
const VK_END: u32 = 0x23;
const VK_PRIOR: u32 = 0x21;
const VK_NEXT: u32 = 0x22;
const VK_DELETE: u32 = 0x2E;
const VK_LMENU: u32 = 0xA4;
const VK_RMENU: u32 = 0xA5;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedHotkey {
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
    vk: u32,
    lone_modifier: bool,
}

fn parse_hotkey(spec: &str) -> Option<ParsedHotkey> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.eq_ignore_ascii_case("RAlt") {
        return Some(ParsedHotkey {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            vk: VK_RMENU,
            lone_modifier: true,
        });
    }
    if trimmed.eq_ignore_ascii_case("LAlt") {
        return Some(ParsedHotkey {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            vk: VK_LMENU,
            lone_modifier: true,
        });
    }

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut win = false;
    let mut key_part: Option<&str> = None;

    for part in trimmed.split('+').map(str::trim) {
        if part.eq_ignore_ascii_case("Ctrl") || part.eq_ignore_ascii_case("Control") {
            ctrl = true;
        } else if part.eq_ignore_ascii_case("Alt") {
            alt = true;
        } else if part.eq_ignore_ascii_case("Shift") {
            shift = true;
        } else if part.eq_ignore_ascii_case("Win") || part.eq_ignore_ascii_case("Super") {
            win = true;
        } else {
            key_part = Some(part);
        }
    }

    let key = key_part?;
    let vk = match key.to_ascii_lowercase().as_str() {
        "pause" | "break" => VK_PAUSE,
        "escape" | "esc" => VK_ESCAPE,
        "space" => VK_SPACE,
        "printscreen" | "prtsc" | "snapshot" => VK_SNAPSHOT,
        "tab" => VK_TAB,
        "enter" | "return" => VK_RETURN,
        "f1" => 0x70,
        "f2" => 0x71,
        "f3" => 0x72,
        "f4" => 0x73,
        "f5" => 0x74,
        "f6" => 0x75,
        "f7" => 0x76,
        "f8" => 0x77,
        "f9" => 0x78,
        "f10" => 0x79,
        "f11" => 0x7A,
        "f12" => 0x7B,
        s if s.len() == 1 => {
            let b = s.as_bytes()[0].to_ascii_uppercase();
            if b.is_ascii_alphanumeric() {
                u32::from(b)
            } else {
                return None;
            }
        }
        _ => return None,
    };

    Some(ParsedHotkey {
        ctrl,
        alt,
        shift,
        win,
        vk,
        lone_modifier: false,
    })
}

fn matches_keydown_hotkey(
    spec: &str,
    vk: u32,
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
) -> bool {
    let Some(hk) = parse_hotkey(spec) else {
        return false;
    };
    !hk.lone_modifier
        && hk.vk == vk
        && hk.ctrl == ctrl
        && hk.alt == alt
        && hk.shift == shift
        && hk.win == win
}

fn matches_lone_tap_hotkey(spec: &str, vk: u32, was_lone_tap: bool) -> bool {
    if !was_lone_tap {
        return false;
    }
    let Some(hk) = parse_hotkey(spec) else {
        return false;
    };
    hk.lone_modifier && hk.vk == vk
}

fn check_screenshot_hotkeys(
    hotkeys: &HotkeyConfig,
    vk: u32,
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
) -> Option<&'static str> {
    if matches_keydown_hotkey(&hotkeys.screenshot_region, vk, ctrl, alt, shift, win) {
        return Some("region");
    }
    if matches_keydown_hotkey(&hotkeys.screenshot_window, vk, ctrl, alt, shift, win) {
        return Some("window");
    }
    if matches_keydown_hotkey(&hotkeys.screenshot_fullscreen, vk, ctrl, alt, shift, win) {
        return Some("fullscreen");
    }
    None
}

pub fn start_daemon(app: AppHandle) {
    let rx = {
        let state = app.state::<AppState>();
        match state.platform.start_hooks() {
            Ok(rx) => rx,
            Err(_) => return,
        }
    };

    std::thread::Builder::new()
        .name("keycraft-daemon-loop".to_string())
        .spawn(move || {
            let mut last_process = String::new();
            let mut last_layout = Language::En;

            while let Ok(evt) = rx.recv() {
                let state = app.state::<AppState>();
                match evt {
                    PlatformEvent::MouseClick => {
                        if let Ok(mut eng) = state.engine.lock() {
                            eng.reset_buffer();
                        }
                    }
                    PlatformEvent::FocusChanged(focus) => {
                        if let Ok(mut eng) = state.engine.lock() {
                            eng.reset_buffer();
                            // Apply per-app pinned starting layout when switching into a different process
                            if !focus.process_name.is_empty()
                                && focus.process_name != last_process
                                && eng.config().enabled
                            {
                                if let Some(rule) = eng.config().find_app_rule(&focus.process_name) {
                                    let desired = match rule.pinned_layout {
                                        PinnedLayout::Keep => None,
                                        PinnedLayout::En => Some(Language::En),
                                        PinnedLayout::Ru => Some(Language::Ru),
                                    };
                                    if let Some(target_lang) = desired {
                                        if target_lang != focus.layout {
                                            let _ = state.platform.set_layout(target_lang);
                                        }
                                    }
                                }
                            }
                        }
                        last_process = focus.process_name;
                    }
                    PlatformEvent::LayoutChanged(layout) => {
                        if layout != last_layout {
                            last_layout = layout;
                            let (enabled, show_badge, duration_ms) = state
                                .engine
                                .lock()
                                .map(|e| {
                                    (
                                        e.config().enabled,
                                        e.config().system.show_caret_badge,
                                        e.config().system.caret_badge_duration_ms,
                                    )
                                })
                                .unwrap_or((true, true, 700));

                            tray::refresh_tray_icon(&app, layout, enabled);
                            if enabled && show_badge {
                                state.platform.show_caret_badge(layout, duration_ms);
                            }
                        }
                    }
                    PlatformEvent::KeyUp {
                        vk_code,
                        was_lone_tap,
                        layout,
                    } => {
                        let hotkeys = state
                            .engine
                            .lock()
                            .map(|e| e.config().hotkeys.clone())
                            .unwrap_or_default();
                        let focus = state.platform.current_focus();
                        let app_ctx = ActiveAppState {
                            process_name: focus.process_name,
                            is_password_field: focus.is_password_field,
                        };

                        if matches_lone_tap_hotkey(&hotkeys.swap_last_symbol, vk_code, was_lone_tap)
                        {
                            run_engine_input(&state, EngineInput::SwapLastSymbol { layout }, &app_ctx);
                        } else if matches_lone_tap_hotkey(
                            &hotkeys.undo_last_action,
                            vk_code,
                            was_lone_tap,
                        ) {
                            run_engine_input(&state, EngineInput::UndoLast, &app_ctx);
                        }
                    }
                    PlatformEvent::KeyDown {
                        vk_code,
                        ch,
                        ctrl,
                        alt,
                        shift,
                        win,
                        layout,
                        ..
                    } => {
                        let (enabled, hotkeys) = state
                            .engine
                            .lock()
                            .map(|e| (e.config().enabled, e.config().hotkeys.clone()))
                            .unwrap_or((true, HotkeyConfig::default()));

                        // Screenshot hotkeys work even if text switching is paused
                        if let Some(mode) =
                            check_screenshot_hotkeys(&hotkeys, vk_code, ctrl, alt, shift, win)
                        {
                            windows_mgr::open_screenshot_overlay_async(
                                app.clone(),
                                mode.to_string(),
                            );
                            continue;
                        }

                        if !enabled {
                            continue;
                        }

                        let focus = state.platform.current_focus();
                        let app_ctx = ActiveAppState {
                            process_name: focus.process_name.clone(),
                            is_password_field: focus.is_password_field,
                        };

                        // Never touch password fields or KeyCraft's own Settings/Sandbox window
                        if app_ctx.is_password_field
                            || app_ctx.process_name.eq_ignore_ascii_case("keycraft.exe")
                        {
                            if let Ok(mut eng) = state.engine.lock() {
                                eng.reset_buffer();
                            }
                            continue;
                        }

                        // Check configured action hotkeys
                        if matches_keydown_hotkey(
                            &hotkeys.undo_last_action,
                            vk_code,
                            ctrl,
                            alt,
                            shift,
                            win,
                        ) {
                            run_engine_input(&state, EngineInput::UndoLast, &app_ctx);
                            continue;
                        }

                        if matches_keydown_hotkey(
                            &hotkeys.swap_last_symbol,
                            vk_code,
                            ctrl,
                            alt,
                            shift,
                            win,
                        ) {
                            run_engine_input(&state, EngineInput::SwapLastSymbol { layout }, &app_ctx);
                            continue;
                        }

                        if matches_keydown_hotkey(
                            &hotkeys.convert_selection_or_word,
                            vk_code,
                            ctrl,
                            alt,
                            shift,
                            win,
                        ) {
                            handle_convert_hotkey(&state, &app_ctx);
                            continue;
                        }

                        if matches_keydown_hotkey(&hotkeys.cycle_case, vk_code, ctrl, alt, shift, win)
                        {
                            handle_cycle_case_hotkey(&state, &app_ctx);
                            continue;
                        }

                        if matches_keydown_hotkey(
                            &hotkeys.add_to_dictionary,
                            vk_code,
                            ctrl,
                            alt,
                            shift,
                            win,
                        ) {
                            handle_add_to_dictionary_hotkey(&state);
                            continue;
                        }

                        // Any other Ctrl/Alt/Win shortcut resets word buffer
                        if ctrl || alt || win {
                            if let Ok(mut eng) = state.engine.lock() {
                                eng.reset_buffer();
                            }
                            continue;
                        }

                        // Map standard editing/navigation keys
                        let input = match vk_code {
                            VK_BACK => EngineInput::Backspace,
                            VK_SPACE => EngineInput::Space { layout },
                            VK_TAB => EngineInput::Tab { layout },
                            VK_RETURN => EngineInput::Enter { layout },
                            VK_LEFT | VK_UP | VK_RIGHT | VK_DOWN | VK_HOME | VK_END | VK_PRIOR
                            | VK_NEXT | VK_DELETE => EngineInput::ResetContext,
                            _ => match ch {
                                Some(c) => EngineInput::Char { ch: c, layout },
                                None => continue,
                            },
                        };

                        run_engine_input(&state, input, &app_ctx);
                    }
                }
            }
        })
        .expect("spawn KeyCraft daemon loop");
}

fn run_engine_input(state: &AppState, input: EngineInput, app_ctx: &ActiveAppState) {
    let (action, config_to_save) = {
        let Ok(mut eng) = state.engine.lock() else {
            return;
        };
        let act = eng.process(input, app_ctx);
        let save_cfg = match &act {
            EngineAction::ReplaceText {
                auto_learned_word: Some(_),
                ..
            } => Some(eng.config().clone()),
            _ => None,
        };
        (act, save_cfg)
    };

    if let Some(cfg) = config_to_save {
        let _ = store::save_config(&cfg);
    }

    match action {
        EngineAction::NoOp => {}
        EngineAction::ReplaceText {
            backspace_count,
            text,
            switch_layout,
            ..
        } => {
            // Brief settle delay so the foreground app finishes inserting the triggering key
            // from Windows' message queue before our synthetic backspaces arrive.
            std::thread::sleep(std::time::Duration::from_millis(12));
            let _ = state
                .platform
                .replace_text(backspace_count, &text, switch_layout);
        }
        EngineAction::InsertText { text, .. } => {
            std::thread::sleep(std::time::Duration::from_millis(12));
            let _ = state.platform.insert_text(&text);
        }
    }
}

fn handle_convert_hotkey(state: &AppState, app_ctx: &ActiveAppState) {
    let engine_arc = Arc::clone(&state.engine);
    let converted_selection = state
        .platform
        .transform_selection(&|selected| {
            let eng = engine_arc.lock().ok()?;
            let (out, lang) = eng.convert_selected_text(selected);
            if out == selected {
                None
            } else {
                Some((out, Some(lang)))
            }
        })
        .unwrap_or(false);

    if !converted_selection {
        run_engine_input(state, EngineInput::ManualConvertLastWord, app_ctx);
    }
}

fn handle_cycle_case_hotkey(state: &AppState, app_ctx: &ActiveAppState) {
    let engine_arc = Arc::clone(&state.engine);
    let cycled_selection = state
        .platform
        .transform_selection(&|selected| {
            let eng = engine_arc.lock().ok()?;
            let out = eng.cycle_selected_case(selected);
            if out == selected {
                None
            } else {
                Some((out, None))
            }
        })
        .unwrap_or(false);

    if !cycled_selection {
        run_engine_input(state, EngineInput::ManualCycleCaseLastWord, app_ctx);
    }
}

fn handle_add_to_dictionary_hotkey(state: &AppState) {
    let Ok(Some(selected)) = state.platform.read_selection_text() else {
        return;
    };
    let word = selected
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphabetic() && c != '-' && c != '\'');
    if word.is_empty() {
        return;
    }
    let cfg_to_save = {
        let Ok(mut eng) = state.engine.lock() else {
            return;
        };
        if eng.add_user_word(word, WordSource::Manual) {
            Some(eng.config().clone())
        } else {
            None
        }
    };
    if let Some(cfg) = cfg_to_save {
        let _ = store::save_config(&cfg);
    }
}
