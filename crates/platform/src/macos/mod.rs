//! Typed macOS 14+ backend architectural skeleton.
//!
//! Planned native bindings:
//! - Input interception: `CGEventTapCreate` (`kCGHIDEventTap`) with custom `kCGEventSourceUserData` recursion marker.
//! - Password protection: `IsSecureEventInputEnabled()` + `AXUIElement` (`kAXSubroleAttribute == AXSecureTextField`).
//! - Layout switching: `TISSelectInputSource` (`Carbon.HIToolbox`).
//! - Screen capture: `ScreenCaptureKit` (`SCStream` / `SCScreenshotManager`).
//! - Caret badge & pinned image: borderless `NSPanel` (`NSWindowLevelFloating`, `ignoresMouseEvents`).

use crossbeam_channel::Receiver;
use keycraft_core_engine::Language;

use crate::{
    CapturedFrame, FocusContext, PlatformBackend, PlatformError, PlatformEvent, PlatformResult,
    RunningAppInfo, SelectionTransform,
};

pub struct MacOsBackend;

impl MacOsBackend {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformBackend for MacOsBackend {
    fn start_hooks(&self) -> PlatformResult<Receiver<PlatformEvent>> {
        Err(PlatformError::UnsupportedPlatform("macOS CGEventTap backend"))
    }

    fn stop_hooks(&self) {}

    fn current_focus(&self) -> FocusContext {
        FocusContext::default()
    }

    fn current_layout(&self) -> Language {
        Language::En
    }

    fn set_layout(&self, _layout: Language) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("macOS TISSelectInputSource"))
    }

    fn replace_text(
        &self,
        _backspace_count: usize,
        _text: &str,
        _switch_layout: Option<Language>,
    ) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("macOS CGEventPost"))
    }

    fn insert_text(&self, _text: &str) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("macOS CGEventPost"))
    }

    fn transform_selection(&self, _transform: SelectionTransform<'_>) -> PlatformResult<bool> {
        Err(PlatformError::UnsupportedPlatform("macOS NSPasteboard"))
    }

    fn read_selection_text(&self) -> PlatformResult<Option<String>> {
        Err(PlatformError::UnsupportedPlatform("macOS NSPasteboard"))
    }

    fn show_caret_badge(&self, _layout: Language, _duration_ms: u32) {}

    fn capture_virtual_screen(&self) -> PlatformResult<CapturedFrame> {
        Err(PlatformError::UnsupportedPlatform("macOS ScreenCaptureKit"))
    }

    fn copy_png_to_clipboard(&self, _png_bytes: &[u8]) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("macOS NSPasteboard PNG"))
    }

    fn pin_screenshot(&self, _png_bytes: Vec<u8>, _screen_x: i32, _screen_y: i32) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("macOS floating NSPanel"))
    }

    fn list_running_apps(&self) -> Vec<RunningAppInfo> {
        Vec::new()
    }

    fn set_autostart(&self, _enabled: bool) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("macOS SMAppService"))
    }

    fn is_autostart_enabled(&self) -> bool {
        false
    }

    fn sync_screenshot_hotkeys(
        &self,
        _enabled: bool,
        _region: &str,
        _window: &str,
        _fullscreen: &str,
    ) -> Vec<String> {
        Vec::new()
    }
}
