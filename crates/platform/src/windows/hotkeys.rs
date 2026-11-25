//! Dedicated Win32 `RegisterHotKey` manager for global screenshot shortcuts.
//!
//! Why `RegisterHotKey` instead of `WH_KEYBOARD_LL` for screenshot hotkeys:
//! - Works reliably across Windows 11 UIPI (even when an elevated/Admin window is focused).
//! - Cleanly intercepts `PrintScreen`, `Alt+PrintScreen`, and `Ctrl+PrintScreen` in win32k
//!   (avoiding `WM_KEYDOWN` omission on `VK_SNAPSHOT` and preventing Windows Snipping Tool conflicts).
//! - Unregisters cleanly the instant the user disables the screenshotter toggle.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::Sender;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PeekMessageW, PostThreadMessageW, MSG, PM_NOREMOVE, WM_APP, WM_HOTKEY, WM_QUIT,
};

use crate::PlatformEvent;

const ID_SHOT_REGION: i32 = 101;
const ID_SHOT_WINDOW: i32 = 102;
const ID_SHOT_FULLSCREEN: i32 = 103;
const WM_APP_SYNC_HOTKEYS: u32 = WM_APP + 17;

#[derive(Debug, Clone, Default)]
struct DesiredHotkeys {
    enabled: bool,
    region: String,
    window: String,
    fullscreen: String,
}

fn parse_win32_hotkey(spec: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut mods = MOD_NOREPEAT;
    let mut key_part: Option<&str> = None;

    for part in trimmed.split('+').map(str::trim) {
        if part.eq_ignore_ascii_case("Ctrl") || part.eq_ignore_ascii_case("Control") {
            mods |= MOD_CONTROL;
        } else if part.eq_ignore_ascii_case("Alt") {
            mods |= MOD_ALT;
        } else if part.eq_ignore_ascii_case("Shift") {
            mods |= MOD_SHIFT;
        } else if part.eq_ignore_ascii_case("Win") || part.eq_ignore_ascii_case("Super") {
            mods |= MOD_WIN;
        } else {
            key_part = Some(part);
        }
    }

    let key = key_part?;
    let vk = match key.to_ascii_lowercase().as_str() {
        "printscreen" | "prtsc" | "snapshot" => 0x2Cu32,
        "pause" | "break" => 0x13u32,
        "escape" | "esc" => 0x1Bu32,
        "space" => 0x20u32,
        "insert" | "ins" => 0x2Du32,
        "delete" | "del" => 0x2Eu32,
        "home" => 0x24u32,
        "end" => 0x23u32,
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

    Some((mods, vk))
}

fn apply_registrations(cfg: &DesiredHotkeys) -> Vec<String> {
    let mut warnings = Vec::new();
    unsafe {
        let _ = UnregisterHotKey(HWND::default(), ID_SHOT_REGION);
        let _ = UnregisterHotKey(HWND::default(), ID_SHOT_WINDOW);
        let _ = UnregisterHotKey(HWND::default(), ID_SHOT_FULLSCREEN);

        if !cfg.enabled {
            return warnings;
        }

        let specs = [
            (ID_SHOT_REGION, "region", cfg.region.as_str()),
            (ID_SHOT_WINDOW, "window", cfg.window.as_str()),
            (ID_SHOT_FULLSCREEN, "fullscreen", cfg.fullscreen.as_str()),
        ];

        for (id, label, spec) in specs {
            if let Some((mods, vk)) = parse_win32_hotkey(spec) {
                if let Err(err) = RegisterHotKey(HWND::default(), id, mods, vk) {
                    warnings.push(format!("{label} ({spec}): {err}"));
                }
            }
        }
    }
    warnings
}

pub struct GlobalHotkeyManager {
    thread_id: Arc<AtomicU32>,
    desired: Arc<Mutex<DesiredHotkeys>>,
    last_warnings: Arc<Mutex<Vec<String>>>,
}

impl GlobalHotkeyManager {
    pub fn start(tx: Sender<PlatformEvent>) -> Self {
        let thread_id = Arc::new(AtomicU32::new(0));
        let desired = Arc::new(Mutex::new(DesiredHotkeys::default()));
        let last_warnings = Arc::new(Mutex::new(Vec::new()));

        let tid_clone = Arc::clone(&thread_id);
        let desired_clone = Arc::clone(&desired);
        let warn_clone = Arc::clone(&last_warnings);
        let (ready_tx, ready_rx) = crossbeam_channel::bounded::<()>(1);

        std::thread::Builder::new()
            .name("keycraft-hotkeys".to_string())
            .spawn(move || {
                unsafe {
                    // Force creation of the thread message queue before publishing thread_id
                    let mut dummy = MSG::default();
                    let _ = PeekMessageW(&mut dummy, HWND::default(), 0, 0, PM_NOREMOVE);
                    let tid = windows::Win32::System::Threading::GetCurrentThreadId();
                    tid_clone.store(tid, Ordering::SeqCst);
                }
                let _ = ready_tx.send(());

                // Apply any config already queued before thread finished starting
                if let Ok(cfg) = desired_clone.lock().map(|g| g.clone()) {
                    let warns = apply_registrations(&cfg);
                    if let Ok(mut w) = warn_clone.lock() {
                        *w = warns;
                    }
                }

                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() } {
                    match msg.message {
                        WM_APP_SYNC_HOTKEYS => {
                            if let Ok(cfg) = desired_clone.lock().map(|g| g.clone()) {
                                let warns = apply_registrations(&cfg);
                                if let Ok(mut w) = warn_clone.lock() {
                                    *w = warns;
                                }
                            }
                        }
                        WM_HOTKEY => {
                            let mode = match msg.wParam.0 as i32 {
                                ID_SHOT_REGION => Some("region"),
                                ID_SHOT_WINDOW => Some("window"),
                                ID_SHOT_FULLSCREEN => Some("fullscreen"),
                                _ => None,
                            };
                            if let Some(m) = mode {
                                let _ = tx.try_send(PlatformEvent::ScreenshotHotkey(m.to_string()));
                            }
                        }
                        _ => {}
                    }
                }

                unsafe {
                    let _ = UnregisterHotKey(HWND::default(), ID_SHOT_REGION);
                    let _ = UnregisterHotKey(HWND::default(), ID_SHOT_WINDOW);
                    let _ = UnregisterHotKey(HWND::default(), ID_SHOT_FULLSCREEN);
                }
            })
            .expect("spawn Win32 RegisterHotKey thread");

        let _ = ready_rx.recv_timeout(std::time::Duration::from_millis(500));

        Self {
            thread_id,
            desired,
            last_warnings,
        }
    }

    pub fn sync(&self, enabled: bool, region: &str, window: &str, fullscreen: &str) -> Vec<String> {
        if let Ok(mut guard) = self.desired.lock() {
            *guard = DesiredHotkeys {
                enabled,
                region: region.to_string(),
                window: window.to_string(),
                fullscreen: fullscreen.to_string(),
            };
        }
        let tid = self.thread_id.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_APP_SYNC_HOTKEYS, WPARAM(0), LPARAM(0));
            }
            // Brief yield so registration completes and warnings reflect latest state
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        self.last_warnings
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn stop(&self) {
        let tid = self.thread_id.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}
