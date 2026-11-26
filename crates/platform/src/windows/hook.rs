//! Low-level Windows keyboard & mouse hook thread (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`).
//!
//! Strict latency & recursion guarantees:
//! - Events with `dwExtraInfo == KEYCRAFT_MAGIC_EXTRA_INFO` are ignored immediately.
//! - Callback NEVER blocks or calls UIAutomation/COM; sends raw events via non-blocking `try_send`.
//! - Tracks lone modifier taps (`LAlt` / `RAlt`) for one-key undo & post-factum symbol swap.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;
use keycraft_core_engine::Language;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx, VK_CAPITAL, VK_CONTROL,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN,
    VK_SHIFT, VK_SNAPSHOT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_QUIT, WM_RBUTTONDOWN,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::windows::hotkeys::{parse_prtsc_combo, PrtScCombo};
use crate::PlatformEvent;

/// Unique marker stamped into `SendInput` `dwExtraInfo` so our hook never recurses on own output.
pub const KEYCRAFT_MAGIC_EXTRA_INFO: usize = 0x4B43_5246_0000_0001;

const VK_S_KEY: u32 = 0x53;
const LANG_RUSSIAN: u16 = 0x19;

type PrtScBindings = Arc<RwLock<Vec<(PrtScCombo, &'static str)>>>;

struct HookContext {
    tx: Sender<PlatformEvent>,
    active_lone_modifier: u32,
    modifier_tainted: bool,
    prtsc_bindings: PrtScBindings,
    intercept_win_shift_s: Arc<AtomicBool>,
    swallowed_prtsc_down: bool,
    swallowed_win_shift_s: bool,
    // Per-side low-level modifier tracking: zero GetAsyncKeyState syscalls during normal typing.
    ll_lctrl: bool,
    ll_rctrl: bool,
    ll_lalt: bool,
    ll_ralt: bool,
    ll_lshift: bool,
    ll_rshift: bool,
    ll_lwin: bool,
    ll_rwin: bool,
}

thread_local! {
    static HOOK_CTX: std::cell::RefCell<Option<HookContext>> = const { std::cell::RefCell::new(None) };
}

fn is_vk_down(vk: u16) -> bool {
    unsafe { (GetAsyncKeyState(i32::from(vk)) as u16 & 0x8000) != 0 }
}

/// Decode virtual key + scan code to a Unicode character AND foreground language OFF the LL hook thread.
/// Uses a clean stack keyboard state (no `GetKeyboardState` syscall) and queries foreground HKL once.
pub fn resolve_key_off_hook(
    vk_code: u32,
    scan_code: u32,
    shift: bool,
    caps_lock: bool,
) -> (Option<char>, Language) {
    unsafe {
        let hwnd = GetForegroundWindow();
        let tid = if hwnd.0.is_null() {
            0
        } else {
            GetWindowThreadProcessId(hwnd, None)
        };
        let hkl = GetKeyboardLayout(tid);
        let primary_lang = ((hkl.0 as usize & 0xFFFF) as u16) & 0x03FF;
        let layout = if primary_lang == LANG_RUSSIAN {
            Language::Ru
        } else {
            Language::En
        };

        let mut kb_state = [0u8; 256];
        if shift {
            kb_state[VK_SHIFT.0 as usize] = 0x80;
        }
        if caps_lock {
            kb_state[VK_CAPITAL.0 as usize] = 0x01;
        }

        let mut buf = [0u16; 8];
        // Flag 0x04 (Windows 10 1607+): do NOT mutate keyboard dead-key state!
        let count = ToUnicodeEx(vk_code, scan_code, &kb_state, &mut buf, 0x04, hkl);
        let ch = if count == 1 {
            char::from_u32(u32::from(buf[0])).filter(|c| !c.is_control())
        } else {
            None
        };
        (ch, layout)
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
        let is_key_down = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN);
        let is_key_up = matches!(msg, WM_KEYUP | WM_SYSKEYUP);
        let is_alt_key = vk == u32::from(VK_LMENU.0) || vk == u32::from(VK_RMENU.0);
        let is_prtsc = vk == u32::from(VK_SNAPSHOT.0);
        let mut swallow_key = false;

        HOOK_CTX.with(|cell| {
            let mut borrow = cell.borrow_mut();
            let Some(ctx) = borrow.as_mut() else {
                return;
            };

            // Keep exact per-side low-level modifier tracking synchronized (zero syscalls on normal typing)
            let is_modifier_key = if vk == u32::from(VK_LCONTROL.0) {
                ctx.ll_lctrl = is_key_down;
                true
            } else if vk == u32::from(VK_RCONTROL.0) {
                ctx.ll_rctrl = is_key_down;
                true
            } else if vk == u32::from(VK_CONTROL.0) {
                ctx.ll_lctrl = is_key_down;
                ctx.ll_rctrl = is_key_down;
                true
            } else if vk == u32::from(VK_LMENU.0) {
                ctx.ll_lalt = is_key_down;
                true
            } else if vk == u32::from(VK_RMENU.0) {
                ctx.ll_ralt = is_key_down;
                true
            } else if vk == u32::from(VK_MENU.0) {
                ctx.ll_lalt = is_key_down;
                ctx.ll_ralt = is_key_down;
                true
            } else if vk == u32::from(VK_LSHIFT.0) {
                ctx.ll_lshift = is_key_down;
                true
            } else if vk == u32::from(VK_RSHIFT.0) {
                ctx.ll_rshift = is_key_down;
                true
            } else if vk == u32::from(VK_SHIFT.0) {
                ctx.ll_lshift = is_key_down;
                ctx.ll_rshift = is_key_down;
                true
            } else if vk == u32::from(VK_LWIN.0) {
                ctx.ll_lwin = is_key_down;
                true
            } else if vk == u32::from(VK_RWIN.0) {
                ctx.ll_rwin = is_key_down;
                true
            } else {
                false
            };

            // Fast-path modifier keys: track lone Alt taps, never stall or wake daemon on regular Shift/Ctrl/Win
            if is_modifier_key {
                if is_key_down {
                    if is_alt_key {
                        if ctx.active_lone_modifier == 0 {
                            ctx.active_lone_modifier = vk;
                            ctx.modifier_tainted = false;
                        }
                    } else if ctx.active_lone_modifier != 0 {
                        ctx.modifier_tainted = true;
                    }
                } else if is_key_up && is_alt_key && ctx.active_lone_modifier == vk {
                    let lone = !ctx.modifier_tainted;
                    ctx.active_lone_modifier = 0;
                    ctx.modifier_tainted = false;
                    if lone {
                        let _ = ctx.tx.try_send(PlatformEvent::KeyUp {
                            vk_code: vk,
                            was_lone_tap: true,
                        });
                    }
                }
                return;
            }

            // Self-heal stale modifier flags only if a modifier is currently marked down
            // (0 GetAsyncKeyState syscalls during regular lowercase typing!)
            if (ctx.ll_lctrl || ctx.ll_rctrl) && !is_vk_down(VK_CONTROL.0) {
                ctx.ll_lctrl = false;
                ctx.ll_rctrl = false;
            }
            if (ctx.ll_lalt || ctx.ll_ralt) && (kb.flags.0 & 0x20) == 0 && !is_vk_down(VK_MENU.0) {
                ctx.ll_lalt = false;
                ctx.ll_ralt = false;
            }
            if (ctx.ll_lshift || ctx.ll_rshift) && !is_vk_down(VK_SHIFT.0) {
                ctx.ll_lshift = false;
                ctx.ll_rshift = false;
            }
            if (ctx.ll_lwin || ctx.ll_rwin) && !is_vk_down(VK_LWIN.0) && !is_vk_down(VK_RWIN.0) {
                ctx.ll_lwin = false;
                ctx.ll_rwin = false;
            }

            let ctrl_down = ctx.ll_lctrl || ctx.ll_rctrl;
            let alt_down = ctx.ll_lalt || ctx.ll_ralt || (kb.flags.0 & 0x20) != 0;
            let shift_down = ctx.ll_lshift || ctx.ll_rshift;
            let win_down = ctx.ll_lwin || ctx.ll_rwin;

            // Intercept PrintScreen combos BEFORE Windows 11's Snipping Tool handler sees VK_SNAPSHOT!
            // Handles normal, key-up-only, and PS/2 fake-Shift (E0 2A E0 37) VK_SNAPSHOT keyboards.
            if is_prtsc {
                if ctx.active_lone_modifier != 0 {
                    ctx.modifier_tainted = true;
                }
                // Extra belt-and-suspenders check only on VK_SNAPSHOT (not regular typing)
                let p_ctrl = ctrl_down || is_vk_down(VK_CONTROL.0);
                let p_alt = alt_down || is_vk_down(VK_MENU.0);
                let p_shift = shift_down || is_vk_down(VK_SHIFT.0);
                let p_win = win_down || is_vk_down(VK_LWIN.0) || is_vk_down(VK_RWIN.0);
                let match_mode = |bindings: &PrtScBindings| -> Option<&'static str> {
                    let guard = bindings.try_read().ok()?;
                    // 1. Exact modifier match
                    if let Some((_, mode)) = guard.iter().find(|(c, _)| {
                        c.ctrl == p_ctrl
                            && c.alt == p_alt
                            && c.shift == p_shift
                            && c.win == p_win
                    }) {
                        return Some(*mode);
                    }
                    // 2. PS/2 hardware fake-Shift fallback for bare PrintScreen
                    if !p_ctrl && !p_alt && !p_win && p_shift {
                        if let Some((_, mode)) = guard
                            .iter()
                            .find(|(c, _)| !c.ctrl && !c.alt && !c.shift && !c.win)
                        {
                            return Some(*mode);
                        }
                    }
                    None
                };

                if is_key_down {
                    if let Some(mode) = match_mode(&ctx.prtsc_bindings) {
                        ctx.swallowed_prtsc_down = true;
                        swallow_key = true;
                        let _ = ctx
                            .tx
                            .try_send(PlatformEvent::ScreenshotHotkey(mode.to_string()));
                        return;
                    }
                } else if is_key_up {
                    if ctx.swallowed_prtsc_down {
                        ctx.swallowed_prtsc_down = false;
                        swallow_key = true;
                        return;
                    }
                    if let Some(mode) = match_mode(&ctx.prtsc_bindings) {
                        swallow_key = true;
                        let _ = ctx
                            .tx
                            .try_send(PlatformEvent::ScreenshotHotkey(mode.to_string()));
                        return;
                    }
                }
            }

            // Intercept Win+Shift+S (sent by Lenovo IdeaPad / ASUS / HP / Dell PrtSc/Scissors keys
            // and LenovoUtilityService SendSnippingScanCode) when PrintScreen capture is active.
            if vk == VK_S_KEY && ctx.intercept_win_shift_s.load(Ordering::Relaxed) {
                if is_key_down && win_down && shift_down && !ctrl_down && !alt_down {
                    ctx.swallowed_win_shift_s = true;
                    swallow_key = true;
                    let _ = ctx
                        .tx
                        .try_send(PlatformEvent::ScreenshotHotkey("region".to_string()));
                    return;
                } else if is_key_up && ctx.swallowed_win_shift_s {
                    ctx.swallowed_win_shift_s = false;
                    swallow_key = true;
                    return;
                }
            }

            if is_key_down {
                if ctx.active_lone_modifier != 0 {
                    ctx.modifier_tainted = true;
                }
                let caps_lock = (GetKeyState(i32::from(VK_CAPITAL.0)) & 0x0001) != 0;
                let evt = PlatformEvent::KeyDown {
                    vk_code: vk,
                    scan_code: kb.scanCode,
                    ctrl: ctrl_down,
                    alt: alt_down,
                    shift: shift_down,
                    win: win_down,
                    caps_lock,
                };
                let _ = ctx.tx.try_send(evt);
            }
        });

        if swallow_key {
            return LRESULT(1);
        }
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
    prtsc_bindings: PrtScBindings,
    intercept_win_shift_s: Arc<AtomicBool>,
}

impl HookThreadHandle {
    pub fn start(tx: Sender<PlatformEvent>, intercept_win_shift_s: Arc<AtomicBool>) -> Self {
        let thread_id = Arc::new(AtomicU32::new(0));
        let running = Arc::new(AtomicBool::new(true));
        let prtsc_bindings: PrtScBindings = Arc::new(RwLock::new(Vec::new()));

        let tid_clone = Arc::clone(&thread_id);
        let run_clone = Arc::clone(&running);
        let bindings_clone = Arc::clone(&prtsc_bindings);
        let snip_flag = Arc::clone(&intercept_win_shift_s);

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
                        prtsc_bindings: bindings_clone,
                        intercept_win_shift_s: snip_flag,
                        swallowed_prtsc_down: false,
                        swallowed_win_shift_s: false,
                        ll_lctrl: false,
                        ll_rctrl: false,
                        ll_lalt: false,
                        ll_ralt: false,
                        ll_lshift: false,
                        ll_rshift: false,
                        ll_lwin: false,
                        ll_rwin: false,
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

        Self {
            thread_id,
            running,
            prtsc_bindings,
            intercept_win_shift_s,
        }
    }

    /// Update which `PrintScreen` (`VK_SNAPSHOT`) modifier combinations should be intercepted
    /// and swallowed in `WH_KEYBOARD_LL` before Windows 11 Snipping Tool receives them.
    pub fn sync_prtsc_bindings(&self, enabled: bool, region: &str, window: &str, fullscreen: &str) {
        let mut next = Vec::new();
        let mut take_over_snipping = false;
        if enabled {
            if let Some(c) = parse_prtsc_combo(region) {
                if !c.ctrl && !c.alt && !c.shift && !c.win {
                    take_over_snipping = true;
                }
                next.push((c, "region"));
            } else if region.eq_ignore_ascii_case("Win+Shift+S") {
                take_over_snipping = true;
            }
            if let Some(c) = parse_prtsc_combo(window) {
                next.push((c, "window"));
            }
            if let Some(c) = parse_prtsc_combo(fullscreen) {
                next.push((c, "fullscreen"));
            }
        }
        self.intercept_win_shift_s
            .store(take_over_snipping, Ordering::Relaxed);
        if let Ok(mut guard) = self.prtsc_bindings.write() {
            *guard = next;
        }
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
