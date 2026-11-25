//! Low-level Windows keyboard & mouse hook thread (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`).
//!
//! Strict latency & recursion guarantees:
//! - Events with `dwExtraInfo == KEYCRAFT_MAGIC_EXTRA_INFO` are ignored immediately.
//! - Callback NEVER blocks or calls UIAutomation/COM; sends raw events via non-blocking `try_send`.
//! - Tracks lone modifier taps (`LAlt` / `RAlt`) for one-key undo & post-factum symbol swap.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use crossbeam_channel::Sender;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardLayout, GetKeyboardState, ToUnicodeEx, VK_CONTROL, VK_LMENU,
    VK_LWIN, VK_MENU, VK_RMENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_QUIT, WM_RBUTTONDOWN,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::windows::focus::query_foreground_layout;
use crate::PlatformEvent;

/// Unique marker stamped into `SendInput` `dwExtraInfo` so our hook never recurses on own output.
pub const KEYCRAFT_MAGIC_EXTRA_INFO: usize = 0x4B43_5246_0000_0001;

struct HookContext {
    tx: Sender<PlatformEvent>,
    active_lone_modifier: u32,
    modifier_tainted: bool,
}

thread_local! {
    static HOOK_CTX: std::cell::RefCell<Option<HookContext>> = const { std::cell::RefCell::new(None) };
}

fn is_vk_down(vk: u16) -> bool {
    unsafe { (GetAsyncKeyState(i32::from(vk)) as u16 & 0x8000) != 0 }
}

/// Translate virtual key + scan code to a Unicode character under the foreground thread's HKL.
fn decode_unicode_char(vk_code: u32, scan_code: u32) -> Option<char> {
    unsafe {
        let mut kb_state = [0u8; 256];
        let _ = GetKeyboardState(&mut kb_state);
        // Sync real-time Shift / CapsLock / AltGr state into kb_state
        if is_vk_down(VK_SHIFT.0) {
            kb_state[VK_SHIFT.0 as usize] = 0x80;
        } else {
            kb_state[VK_SHIFT.0 as usize] = 0;
        }
        // Do not decode printable characters when Ctrl or Alt is held (shortcuts)
        if is_vk_down(VK_CONTROL.0) || is_vk_down(VK_MENU.0) || is_vk_down(VK_LWIN.0) || is_vk_down(VK_RWIN.0) {
            return None;
        }

        let hwnd = GetForegroundWindow();
        let tid = if hwnd.0.is_null() {
            0
        } else {
            GetWindowThreadProcessId(hwnd, None)
        };
        let hkl = GetKeyboardLayout(tid);

        let mut buf = [0u16; 8];
        // Flag 0x04 (Windows 10 1607+): do NOT change keyboard dead-key state!
        let count = ToUnicodeEx(vk_code, scan_code, &kb_state, &mut buf, 0x04, hkl);
        if count == 1 {
            char::from_u32(u32::from(buf[0])).filter(|c| !c.is_control())
        } else {
            None
        }
    }
}

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && lparam.0 != 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

        // Immediately ignore synthetic inputs emitted by KeyCraft itself!
        if kb.dwExtraInfo == KEYCRAFT_MAGIC_EXTRA_INFO {
            return CallNextHookEx(HHOOK::default(), code, wparam, lparam);
        }

        let msg = wparam.0 as u32;
        let vk = kb.vkCode;
        let is_alt_key = vk == u32::from(VK_LMENU.0) || vk == u32::from(VK_RMENU.0);

        HOOK_CTX.with(|cell| {
            let mut borrow = cell.borrow_mut();
            let Some(ctx) = borrow.as_mut() else {
                return;
            };

            match msg {
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    if is_alt_key {
                        if ctx.active_lone_modifier == 0 {
                            ctx.active_lone_modifier = vk;
                            ctx.modifier_tainted = false;
                        }
                    } else if ctx.active_lone_modifier != 0 {
                        ctx.modifier_tainted = true;
                    }

                    let layout = query_foreground_layout();
                    let ch = decode_unicode_char(vk, kb.scanCode);
                    let evt = PlatformEvent::KeyDown {
                        vk_code: vk,
                        scan_code: kb.scanCode,
                        ch,
                        ctrl: is_vk_down(VK_CONTROL.0),
                        alt: is_vk_down(VK_MENU.0),
                        shift: is_vk_down(VK_SHIFT.0),
                        win: is_vk_down(VK_LWIN.0) || is_vk_down(VK_RWIN.0),
                        layout,
                    };
                    let _ = ctx.tx.try_send(evt);
                }
                WM_KEYUP | WM_SYSKEYUP => {
                    let was_lone_tap = if is_alt_key && ctx.active_lone_modifier == vk {
                        let lone = !ctx.modifier_tainted;
                        ctx.active_lone_modifier = 0;
                        ctx.modifier_tainted = false;
                        lone
                    } else {
                        false
                    };
                    // Only emit KeyUp into the daemon channel for lone modifier taps (zero noise on regular typing).
                    if was_lone_tap {
                        let layout = query_foreground_layout();
                        let _ = ctx.tx.try_send(PlatformEvent::KeyUp {
                            vk_code: vk,
                            was_lone_tap: true,
                            layout,
                        });
                    }
                }
                _ => {}
            }
        });
    }

    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let msg = wparam.0 as u32;
        if matches!(msg, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN) {
            HOOK_CTX.with(|cell| {
                if let Some(ctx) = cell.borrow_mut().as_mut() {
                    if ctx.active_lone_modifier != 0 {
                        ctx.modifier_tainted = true;
                    }
                    let _ = ctx.tx.try_send(PlatformEvent::MouseClick);
                }
            });
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

pub struct HookThreadHandle {
    thread_id: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
}

impl HookThreadHandle {
    pub fn start(tx: Sender<PlatformEvent>) -> Self {
        let thread_id = Arc::new(AtomicU32::new(0));
        let running = Arc::new(AtomicBool::new(true));

        let tid_clone = Arc::clone(&thread_id);
        let run_clone = Arc::clone(&running);

        std::thread::Builder::new()
            .name("keycraft-ll-hooks".to_string())
            .spawn(move || {
                let current_tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
                tid_clone.store(current_tid, Ordering::SeqCst);

                HOOK_CTX.with(|cell| {
                    *cell.borrow_mut() = Some(HookContext {
                        tx,
                        active_lone_modifier: 0,
                        modifier_tainted: false,
                    });
                });

                unsafe {
                    let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
                        .unwrap_or_default();
                    let ms_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)
                        .unwrap_or_default();

                    let mut msg = MSG::default();
                    while run_clone.load(Ordering::Relaxed)
                        && GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool()
                    {}

                    if !kb_hook.is_invalid() {
                        let _ = UnhookWindowsHookEx(kb_hook);
                    }
                    if !ms_hook.is_invalid() {
                        let _ = UnhookWindowsHookEx(ms_hook);
                    }
                }

                HOOK_CTX.with(|cell| {
                    *cell.borrow_mut() = None;
                });
            })
            .expect("spawn low-level hook thread");

        Self { thread_id, running }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        let tid = self.thread_id.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}
