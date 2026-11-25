//! Typed Linux (X11 / Wayland) backend architectural skeleton.
//!
//! Planned native bindings:
//! - X11: `XRecord` / `XInput2` event capture, `XTestFakeKeyEvent` injection, `XkbLockGroup` layout switch, AT-SPI2 password detection.
//! - Wayland: `evdev`/`uinput` or compositor input-method protocol, `xdg-desktop-portal` Screenshot API.

use crossbeam_channel::Receiver;
use keycraft_core_engine::Language;

use crate::{
    CapturedFrame, FocusContext, PlatformBackend, PlatformError, PlatformEvent, PlatformResult,
    RunningAppInfo, SelectionTransform,
};

pub struct LinuxBackend;

impl LinuxBackend {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformBackend for LinuxBackend {
    fn start_hooks(&self) -> PlatformResult<Receiver<PlatformEvent>> {
        Err(PlatformError::UnsupportedPlatform("Linux X11/Wayland hook backend"))
    }

    fn stop_hooks(&self) {}

    fn current_focus(&self) -> FocusContext {
        FocusContext::default()
    }

    fn current_layout(&self) -> Language {
        Language::En
    }

    fn set_layout(&self, _layout: Language) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("Linux XkbLockGroup"))
    }

    fn replace_text(
        &self,
        _backspace_count: usize,
        _text: &str,
        _switch_layout: Option<Language>,
    ) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("Linux XTest/uinput injection"))
    }

    fn insert_text(&self, _text: &str) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("Linux XTest/uinput injection"))
    }

    fn transform_selection(&self, _transform: SelectionTransform<'_>) -> PlatformResult<bool> {
        Err(PlatformError::UnsupportedPlatform("Linux clipboard selection"))
    }

    fn read_selection_text(&self) -> PlatformResult<Option<String>> {
        Err(PlatformError::UnsupportedPlatform("Linux clipboard selection"))
    }

    fn show_caret_badge(&self, _layout: Language, _duration_ms: u32) {}

    fn capture_virtual_screen(&self) -> PlatformResult<CapturedFrame> {
        Err(PlatformError::UnsupportedPlatform("Linux X11/Portal screenshot"))
    }

    fn copy_png_to_clipboard(&self, _png_bytes: &[u8]) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("Linux PNG clipboard"))
    }

    fn pin_screenshot(&self, _png_bytes: Vec<u8>, _screen_x: i32, _screen_y: i32) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("Linux floating window"))
    }

    fn list_running_apps(&self) -> Vec<RunningAppInfo> {
        Vec::new()
    }

    fn set_autostart(&self, _enabled: bool) -> PlatformResult<()> {
        Err(PlatformError::UnsupportedPlatform("Linux XDG autostart"))
    }

    fn is_autostart_enabled(&self) -> bool {
        false
    }
}
