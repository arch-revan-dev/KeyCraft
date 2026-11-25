//! Synthetic text injection (`SendInput` with `dwExtraInfo` anti-recursion marker),
//! OS keyboard layout switching (`WM_INPUTLANGCHANGEREQUEST`), and safe selection conversion.

use keycraft_core_engine::Language;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardLayoutList, SendInput, HKL, INPUT, INPUT_0, INPUT_KEYBOARD,
    KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK,
    VK_CONTROL, VK_LMENU, VK_LSHIFT, VK_MENU, VK_RMENU, VK_RSHIFT, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
};

use crate::windows::clipboard;
use crate::windows::hook::KEYCRAFT_MAGIC_EXTRA_INFO;
use crate::{PlatformError, PlatformResult, SelectionTransform};

const LANG_RUSSIAN: u16 = 0x19;
const LANG_ENGLISH: u16 = 0x09;

fn make_vk_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: KEYCRAFT_MAGIC_EXTRA_INFO,
            },
        },
    }
}

fn make_unicode_inputs(text: &str, out: &mut Vec<INPUT>) {
    for unit in text.encode_utf16() {
        out.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: KEYCRAFT_MAGIC_EXTRA_INFO,
                },
            },
        });
        out.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYCRAFT_MAGIC_EXTRA_INFO,
                },
            },
        });
    }
}

fn send_batch(inputs: &[INPUT]) -> PlatformResult<()> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(PlatformError::OsError("SendInput sent 0 events".to_string()));
    }
    Ok(())
}

/// Release any physically held modifier keys temporarily before injecting synthetic keys.
fn release_held_modifiers(out: &mut Vec<INPUT>) {
    for vk in [VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_CONTROL, VK_MENU, VK_LMENU, VK_RMENU] {
        let down = unsafe { (GetAsyncKeyState(i32::from(vk.0)) as u16 & 0x8000) != 0 };
        if down {
            out.push(make_vk_input(vk, true));
        }
    }
}

/// Find the installed Windows `HKL` for Russian or English.
fn find_hkl_for_language(target: Language) -> Option<HKL> {
    let want_primary = match target {
        Language::Ru => LANG_RUSSIAN,
        Language::En => LANG_ENGLISH,
    };
    unsafe {
        let count = GetKeyboardLayoutList(None);
        if count <= 0 {
            return None;
        }
        let mut hkls = vec![HKL::default(); count as usize];
        let filled = GetKeyboardLayoutList(Some(&mut hkls));
        for &hkl in hkls.iter().take(filled.max(0) as usize) {
            let lang_id = (hkl.0 as usize & 0xFFFF) as u16;
            if (lang_id & 0x03FF) == want_primary {
                return Some(hkl);
            }
        }
    }
    None
}

pub fn set_foreground_layout(target: Language) -> PlatformResult<()> {
    let Some(hkl) = find_hkl_for_language(target) else {
        return Ok(());
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            let _ = PostMessageW(hwnd, WM_INPUTLANGCHANGEREQUEST, WPARAM(0), LPARAM(hkl.0 as isize));
        }
    }
    Ok(())
}

pub fn replace_text(
    backspace_count: usize,
    text: &str,
    switch_layout: Option<Language>,
) -> PlatformResult<()> {
    let mut inputs = Vec::with_capacity(backspace_count * 2 + text.len() * 2 + 8);
    release_held_modifiers(&mut inputs);

    for _ in 0..backspace_count {
        inputs.push(make_vk_input(VK_BACK, false));
        inputs.push(make_vk_input(VK_BACK, true));
    }
    make_unicode_inputs(text, &mut inputs);
    send_batch(&inputs)?;

    if let Some(lang) = switch_layout {
        let _ = set_foreground_layout(lang);
    }
    Ok(())
}

pub fn insert_text(text: &str) -> PlatformResult<()> {
    let mut inputs = Vec::with_capacity(text.len() * 2 + 4);
    make_unicode_inputs(text, &mut inputs);
    send_batch(&inputs)
}

fn send_ctrl_key(vk_letter: u16) -> PlatformResult<()> {
    let mut inputs = Vec::with_capacity(12);
    release_held_modifiers(&mut inputs);
    inputs.push(make_vk_input(VK_CONTROL, false));
    inputs.push(make_vk_input(VIRTUAL_KEY(vk_letter), false));
    inputs.push(make_vk_input(VIRTUAL_KEY(vk_letter), true));
    inputs.push(make_vk_input(VK_CONTROL, true));
    send_batch(&inputs)
}

/// Copy currently selected text using `Ctrl+C` while preserving prior clipboard text.
pub fn read_selection_text() -> PlatformResult<Option<String>> {
    let prev_clip = clipboard::get_text().unwrap_or(None);
    let seq_before = clipboard::sequence_number();

    send_ctrl_key(u16::from(b'C'))?;

    let mut changed = false;
    for _ in 0..16 {
        std::thread::sleep(std::time::Duration::from_millis(12));
        if clipboard::sequence_number() != seq_before {
            changed = true;
            break;
        }
    }

    let selected = if changed {
        clipboard::get_text().unwrap_or(None).filter(|s| !s.is_empty())
    } else {
        None
    };

    if let Some(prev) = prev_clip {
        let _ = clipboard::set_text(&prev);
    }

    Ok(selected)
}

/// Transform currently selected text in-place via clipboard (`Ctrl+C` -> transform -> `Ctrl+V`)
/// and restore original clipboard text afterwards.
pub fn transform_selection(transform: SelectionTransform<'_>) -> PlatformResult<bool> {
    let prev_clip = clipboard::get_text().unwrap_or(None);
    let seq_before = clipboard::sequence_number();

    send_ctrl_key(u16::from(b'C'))?;

    let mut changed = false;
    for _ in 0..16 {
        std::thread::sleep(std::time::Duration::from_millis(12));
        if clipboard::sequence_number() != seq_before {
            changed = true;
            break;
        }
    }

    if !changed {
        return Ok(false);
    }

    let Some(selected) = clipboard::get_text().unwrap_or(None) else {
        return Ok(false);
    };
    if selected.is_empty() {
        return Ok(false);
    }

    let Some((replacement, switch_layout)) = transform(&selected) else {
        if let Some(prev) = prev_clip {
            let _ = clipboard::set_text(&prev);
        }
        return Ok(false);
    };

    clipboard::set_text(&replacement)?;
    send_ctrl_key(u16::from(b'V'))?;

    if let Some(lang) = switch_layout {
        let _ = set_foreground_layout(lang);
    }

    // Brief settle delay before restoring prior clipboard so target app finishes reading Ctrl+V
    if let Some(prev) = prev_clip {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(120));
            let _ = clipboard::set_text(&prev);
        });
    }

    Ok(true)
}
