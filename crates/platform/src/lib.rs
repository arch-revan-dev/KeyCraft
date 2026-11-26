//! Clean cross-platform OS abstraction layer (`PlatformBackend`) for KeyCraft.
//!
//! Primary production implementation: Windows 11 (`src/windows/`).
//! Typed, non-panicking architectural modules prepared for macOS 14+ (`src/macos/`)
//! and Linux X11/Wayland (`src/linux/`).

use crossbeam_channel::Receiver;
use keycraft_core_engine::Language;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("platform operation is not supported on this OS yet: {0}")]
    UnsupportedPlatform(&'static str),
    #[error("OS API error: {0}")]
    OsError(String),
    #[error("capture failed: {0}")]
    CaptureError(String),
    #[error("clipboard error: {0}")]
    ClipboardError(String),
}

pub type PlatformResult<T> = Result<T, PlatformError>;

/// Screen-space rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Active window & security context snapshot cached by the focus monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusContext {
    pub process_name: String,
    pub window_title: String,
    pub is_password_field: bool,
    pub layout: Language,
    pub caret_pos: Option<(i32, i32)>,
}

impl Default for FocusContext {
    fn default() -> Self {
        Self {
            process_name: String::new(),
            window_title: String::new(),
            is_password_field: false,
            layout: Language::En,
            caret_pos: None,
        }
    }
}

/// Summary of a running desktop application for the App Rules picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunningAppInfo {
    pub process_name: String,
    pub window_title: String,
}

/// Raw low-level events pushed from the OS hook thread into the daemon channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    KeyDown {
        vk_code: u32,
        scan_code: u32,
        ctrl: bool,
        alt: bool,
        shift: bool,
        win: bool,
        caps_lock: bool,
    },
    KeyUp {
        vk_code: u32,
        was_lone_tap: bool,
    },
    MouseClick,
    FocusChanged(FocusContext),
    LayoutChanged(Language),
    ScreenshotHotkey(String),
}

/// Captured RGBA image buffer plus virtual screen origin and window rects for snap capture.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
    pub png_bytes: Vec<u8>,
    pub active_window_rect: Option<ScreenRect>,
    pub visible_windows: Vec<ScreenRect>,
}

/// Callback type used to transform selected text in-place via clipboard.
pub type SelectionTransform<'a> = &'a dyn Fn(&str) -> Option<(String, Option<Language>)>;

/// Unified platform trait implemented by Windows 11 (and future macOS / Linux backends).
pub trait PlatformBackend: Send + Sync {
    /// Start low-level keyboard/mouse hooks and focus monitoring, returning an event receiver.
    fn start_hooks(&self) -> PlatformResult<Receiver<PlatformEvent>>;

    /// Stop background hooks cleanly.
    fn stop_hooks(&self);

    /// Read latest cached focus & password-field state (lock-free / sub-microsecond).
    fn current_focus(&self) -> FocusContext;

    /// Query current active input language of the foreground thread.
    fn current_layout(&self) -> Language;

    /// Translate a virtual key + scan code into a Unicode character and active layout OFF the hook thread.
    fn resolve_key(
        &self,
        _vk_code: u32,
        _scan_code: u32,
        _shift: bool,
        _caps_lock: bool,
    ) -> (Option<char>, Language) {
        (None, self.current_layout())
    }

    /// Switch the foreground window's keyboard layout (`RU` / `EN`).
    fn set_layout(&self, layout: Language) -> PlatformResult<()>;

    /// Atomically erase `backspace_count` characters and inject UTF-8 `text`
    /// using synthetic markers so the hook never recurses.
    fn replace_text(
        &self,
        backspace_count: usize,
        text: &str,
        switch_layout: Option<Language>,
    ) -> PlatformResult<()>;

    /// Insert text without backspacing (used for smart list continuation).
    fn insert_text(&self, text: &str) -> PlatformResult<()>;

    /// Copy current selection via synthetic `Ctrl+C`, transform it with `transform`,
    /// paste back via `Ctrl+V`, and restore prior clipboard text. Returns `Ok(true)` if selection existed.
    fn transform_selection(&self, transform: SelectionTransform<'_>) -> PlatformResult<bool>;

    /// Read current text selection via synthetic `Ctrl+C` (restoring prior clipboard) for "Add to dictionary".
    fn read_selection_text(&self) -> PlatformResult<Option<String>>;

    /// Show lightweight native layout badge (`RU` / `EN`) near caret/cursor for `duration_ms`.
    fn show_caret_badge(&self, layout: Language, duration_ms: u32);

    /// Capture entire virtual desktop (multi-monitor, DPI-aware) as PNG + window geometry.
    fn capture_virtual_screen(&self) -> PlatformResult<CapturedFrame>;

    /// Copy PNG image bytes to the OS clipboard as lossless bitmap/PNG.
    fn copy_png_to_clipboard(&self, png_bytes: &[u8]) -> PlatformResult<()>;

    /// Open a lightweight native always-on-top floating window displaying a pinned PNG screenshot.
    fn pin_screenshot(&self, png_bytes: Vec<u8>, screen_x: i32, screen_y: i32) -> PlatformResult<()>;

    /// Enumerate visible desktop windows for the App Rules picker.
    fn list_running_apps(&self) -> Vec<RunningAppInfo>;

    /// Enable or disable OS autostart for the current executable.
    fn set_autostart(&self, enabled: bool) -> PlatformResult<()>;

    /// Check whether OS autostart is currently enabled.
    fn is_autostart_enabled(&self) -> bool;

    /// Register or unregister global OS screenshot hotkeys (`RegisterHotKey` on Windows).
    /// Returns a list of any registration warnings (empty if all succeeded).
    fn sync_screenshot_hotkeys(
        &self,
        enabled: bool,
        region: &str,
        window: &str,
        fullscreen: &str,
    ) -> Vec<String>;

    /// Check whether Windows 11's built-in Snipping Tool is currently hijacking `PrintScreen`.
    fn is_os_snipping_prtsc_enabled(&self) -> bool;

    /// Toggle Windows 11's `PrintScreenKeyForSnippingEnabled` setting so `PrintScreen` is freed for KeyCraft.
    fn set_os_snipping_prtsc_enabled(&self, enabled: bool) -> PlatformResult<()>;
}

/// Ensure only a single KeyCraft daemon instance runs per user session.
pub fn acquire_single_instance_guard() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::autostart::acquire_single_instance_mutex()
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

/// Construct the native backend for the current target OS.
pub fn create_platform_backend() -> PlatformResult<Box<dyn PlatformBackend>> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsBackend::new()?))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOsBackend::new()))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxBackend::new()))
    }
}
