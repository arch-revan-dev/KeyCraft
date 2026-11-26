//! Windows 11 active window, process name, keyboard layout (HKL), caret position,
//! and UIAutomation password-field protection (Zero-Keylogger Architecture).

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;
use keycraft_core_engine::Language;
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK,
    UIA_IsPasswordPropertyId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetMessageW,
    GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, PostMessageW, EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, GWL_EXSTYLE,
    GUITHREADINFO, MSG, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_CLOSE,
    WS_EX_TOOLWINDOW,
};

use crate::windows::hook::KEYCRAFT_MAGIC_EXTRA_INFO;
use crate::{FocusContext, PlatformEvent, RunningAppInfo};

const LANG_RUSSIAN: u16 = 0x19;

/// Query active keyboard layout (`RU` or `EN`) of the current foreground thread.
pub fn query_foreground_layout() -> Language {
    unsafe {
        let hwnd = GetForegroundWindow();
        let tid = if hwnd.0.is_null() {
            0
        } else {
            GetWindowThreadProcessId(hwnd, None)
        };
        let hkl = GetKeyboardLayout(tid);
        let lang_id = (hkl.0 as usize & 0xFFFF) as u16;
        let primary_lang = lang_id & 0x03FF;
        if primary_lang == LANG_RUSSIAN {
            Language::Ru
        } else {
            Language::En
        }
    }
}

/// Query caret screen coordinates for the foreground window, falling back to mouse cursor position.
pub fn query_caret_or_cursor_pos() -> (i32, i32) {
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            let tid = GetWindowThreadProcessId(hwnd, None);
            let mut info = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            if GetGUIThreadInfo(tid, &mut info).is_ok() && !info.hwndCaret.0.is_null() {
                let mut pt = POINT {
                    x: info.rcCaret.left,
                    y: info.rcCaret.bottom,
                };
                if ClientToScreen(info.hwndCaret, &mut pt).as_bool() && (pt.x != 0 || pt.y != 0) {
                    return (pt.x, pt.y);
                }
            }
        }
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        (cursor.x, cursor.y)
    }
}

pub fn process_name_for_hwnd(hwnd: HWND) -> String {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return String::new();
        }
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buf = vec![0u16; 512];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok();
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        if !ok || size == 0 {
            return String::new();
        }
        let full = String::from_utf16_lossy(&buf[..size as usize]);
        std::path::Path::new(&full)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

pub fn window_title_for_hwnd(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len as usize) + 1];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

/// Inspect focused UI element via UIAutomation to detect PasswordBox / Secure Input fields.
/// Uses strictly fast boolean properties (no slow cross-process name/tree walks).
fn check_uia_is_password(uia: &IUIAutomation) -> bool {
    unsafe {
        let Ok(elem) = uia.GetFocusedElement() else {
            return false;
        };
        if let Ok(is_pwd) = elem.CurrentIsPassword() {
            if is_pwd.as_bool() {
                return true;
            }
        }
        if let Ok(val) = elem.GetCurrentPropertyValue(UIA_IsPasswordPropertyId) {
            if let Ok(b) = bool::try_from(&val) {
                if b {
                    return true;
                }
            }
        }
        false
    }
}

/// Inspect active window and update shared `FocusContext` cache.
pub fn refresh_focus_context(
    uia: Option<&IUIAutomation>,
    cache: &Arc<RwLock<FocusContext>>,
) -> FocusContext {
    let hwnd = unsafe { GetForegroundWindow() };
    let process_name = if hwnd.0.is_null() {
        String::new()
    } else {
        process_name_for_hwnd(hwnd)
    };
    let window_title = if hwnd.0.is_null() {
        String::new()
    } else {
        window_title_for_hwnd(hwnd)
    };
    let is_password_field = uia.is_some_and(check_uia_is_password);
    let layout = query_foreground_layout();
    let caret_pos = Some(query_caret_or_cursor_pos());

    let ctx = FocusContext {
        process_name,
        window_title,
        is_password_field,
        layout,
        caret_pos,
    };

    if let Ok(mut guard) = cache.write() {
        *guard = ctx.clone();
    }
    ctx
}

thread_local! {
    static FOCUS_NOTIFY_TX: std::cell::RefCell<Option<Sender<()>>> = const { std::cell::RefCell::new(None) };
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    FOCUS_NOTIFY_TX.with(|cell| {
        if let Some(tx) = cell.borrow().as_ref() {
            let _ = tx.try_send(());
        }
    });
}

/// Check whether the foreground window is specifically Windows 11's fullscreen Snipping capture overlay
/// (e.g. launched directly via OEM ACPI/WMI daemon like Lenovo FnHotkeyUtility bypassing keyboard hooks).
/// Deliberately ignores the main Snipping Tool editor window so users can still open Snipping Tool manually.
fn is_windows_snipping_overlay(process_name: &str, window_title: &str) -> bool {
    let is_snip_proc = process_name.eq_ignore_ascii_case("SnippingTool.exe")
        || process_name.eq_ignore_ascii_case("ScreenClippingHost.exe");
    if !is_snip_proc {
        return false;
    }
    let t = window_title.trim();
    t.eq_ignore_ascii_case("Select area to snip")
        || t.eq_ignore_ascii_case("Screen Snipping")
        || t.eq_ignore_ascii_case("Выберите область для фрагмента")
        || t.eq_ignore_ascii_case("Фрагмент экрана")
        || t.eq_ignore_ascii_case("Набросок на фрагменте экрана")
        || process_name.eq_ignore_ascii_case("ScreenClippingHost.exe")
}

/// Cleanly dismiss Windows 11's Snipping capture overlay via Escape + WM_CLOSE.
fn dismiss_os_snipping_overlay(hwnd: HWND) {
    unsafe {
        let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
        let inputs = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_ESCAPE,
                        wScan: 0,
                        dwFlags: Default::default(),
                        time: 0,
                        dwExtraInfo: KEYCRAFT_MAGIC_EXTRA_INFO,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_ESCAPE,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: KEYCRAFT_MAGIC_EXTRA_INFO,
                    },
                },
            },
        ];
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Spawn dedicated WinEventHook + COM UIAutomation worker threads so hook callbacks NEVER stall.
pub fn spawn_focus_monitor(
    cache: Arc<RwLock<FocusContext>>,
    event_tx: Sender<PlatformEvent>,
    intercept_os_snipping: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    let (ping_tx, ping_rx) = crossbeam_channel::bounded::<()>(8);

    // Worker thread owning COM MTA + IUIAutomation instance
    let cache_worker = Arc::clone(&cache);
    let event_tx_worker = event_tx.clone();
    std::thread::Builder::new()
        .name("keycraft-uia-worker".to_string())
        .spawn(move || {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            let uia: Option<IUIAutomation> = unsafe {
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
            };

            let mut prev = refresh_focus_context(uia.as_ref(), &cache_worker);
            let _ = event_tx_worker.try_send(PlatformEvent::FocusChanged(prev.clone()));
            let mut last_uia_check = std::time::Instant::now();
            let mut last_snip_redirect = std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(5))
                .unwrap_or_else(std::time::Instant::now);

            // Poll layout gently every 250ms while waiting for WinEvent pings; debounce UIA to avoid RPC storms.
            loop {
                match ping_rx.recv_timeout(std::time::Duration::from_millis(250)) {
                    Ok(()) => {
                        // Fast-path check BEFORE debounce: catch OEM-launched Windows Snipping overlay immediately
                        if intercept_os_snipping.load(Ordering::Relaxed)
                            && last_snip_redirect.elapsed() > std::time::Duration::from_millis(800)
                        {
                            let fg = unsafe { GetForegroundWindow() };
                            if !fg.0.is_null() {
                                let proc_name = process_name_for_hwnd(fg);
                                let title = window_title_for_hwnd(fg);
                                if is_windows_snipping_overlay(&proc_name, &title) {
                                    last_snip_redirect = std::time::Instant::now();
                                    dismiss_os_snipping_overlay(fg);
                                    // Wait briefly for Windows dimmed capture layer to clear before KeyCraft captures
                                    std::thread::sleep(std::time::Duration::from_millis(160));
                                    let _ = event_tx_worker.try_send(
                                        PlatformEvent::ScreenshotHotkey("region".to_string()),
                                    );
                                    continue;
                                }
                            }
                        }

                        // Coalesce burst focus notifications from browsers/lists
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        while ping_rx.try_recv().is_ok() {}
                        if last_uia_check.elapsed() < std::time::Duration::from_millis(120) {
                            continue;
                        }
                        last_uia_check = std::time::Instant::now();
                        let next = refresh_focus_context(uia.as_ref(), &cache_worker);
                        if next.process_name != prev.process_name
                            || next.is_password_field != prev.is_password_field
                            || next.window_title != prev.window_title
                        {
                            let _ = event_tx_worker
                                .try_send(PlatformEvent::FocusChanged(next.clone()));
                        }
                        if next.layout != prev.layout {
                            let _ = event_tx_worker.try_send(PlatformEvent::LayoutChanged(next.layout));
                        }
                        prev = next;
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        let current_layout = query_foreground_layout();
                        if current_layout != prev.layout {
                            prev.layout = current_layout;
                            if let Ok(mut guard) = cache_worker.write() {
                                guard.layout = current_layout;
                            }
                            let _ = event_tx_worker
                                .try_send(PlatformEvent::LayoutChanged(current_layout));
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                }
            }

            unsafe {
                CoUninitialize();
            }
        })
        .expect("spawn UIA worker");

    // WinEventHook message pump thread
    std::thread::Builder::new()
        .name("keycraft-winevent-pump".to_string())
        .spawn(move || {
            FOCUS_NOTIFY_TX.with(|cell| {
                *cell.borrow_mut() = Some(ping_tx);
            });

            unsafe {
                let h_fg = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                );
                let h_focus = SetWinEventHook(
                    EVENT_OBJECT_FOCUS,
                    EVENT_OBJECT_FOCUS,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                );

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {}

                if !h_fg.is_invalid() {
                    let _ = UnhookWinEvent(h_fg);
                }
                if !h_focus.is_invalid() {
                    let _ = UnhookWinEvent(h_focus);
                }
            }
        })
        .expect("spawn WinEvent pump")
}

/// Enumerate currently open top-level desktop applications for the App Rules UI picker.
pub fn list_running_apps() -> Vec<RunningAppInfo> {
    struct EnumState {
        seen: HashSet<String>,
        apps: Vec<RunningAppInfo>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
            return BOOL(1);
        }
        let title = window_title_for_hwnd(hwnd);
        if title.is_empty() {
            return BOOL(1);
        }
        let proc_name = process_name_for_hwnd(hwnd);
        if proc_name.is_empty() || proc_name.eq_ignore_ascii_case("keycraft.exe") {
            return BOOL(1);
        }
        let state = &mut *(lparam.0 as *mut EnumState);
        let key = proc_name.to_ascii_lowercase();
        if state.seen.insert(key) {
            state.apps.push(RunningAppInfo {
                process_name: proc_name,
                window_title: title,
            });
        }
        BOOL(1)
    }

    let mut state = EnumState {
        seen: HashSet::new(),
        apps: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM((&mut state as *mut EnumState) as isize));
    }
    state
        .apps
        .sort_by_key(|a| a.process_name.to_lowercase());
    state.apps
}

#[allow(dead_code)]
fn _rect_non_empty(r: &RECT) -> bool {
    r.right > r.left && r.bottom > r.top
}
