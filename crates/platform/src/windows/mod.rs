//! Full Windows 11 implementation of `PlatformBackend`.

pub mod autostart;
pub mod badge;
pub mod capture;
pub mod clipboard;
pub mod focus;
pub mod hook;
pub mod injector;
pub mod pin_window;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crossbeam_channel::{bounded, Receiver};
use keycraft_core_engine::Language;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

use crate::{
    CapturedFrame, FocusContext, PlatformBackend, PlatformEvent, PlatformResult, RunningAppInfo,
    SelectionTransform,
};

pub struct WindowsBackend {
    focus_cache: Arc<RwLock<FocusContext>>,
    hook_handle: Mutex<Option<hook::HookThreadHandle>>,
    intercept_prtsc: Arc<AtomicBool>,
}

impl WindowsBackend {
    pub fn new() -> PlatformResult<Self> {
        // Ensure accurate physical-pixel coordinates across mixed-DPI multi-monitor setups.
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        Ok(Self {
            focus_cache: Arc::new(RwLock::new(FocusContext::default())),
            hook_handle: Mutex::new(None),
            intercept_prtsc: Arc::new(AtomicBool::new(true)),
        })
    }
}

impl PlatformBackend for WindowsBackend {
    fn start_hooks(&self) -> PlatformResult<Receiver<PlatformEvent>> {
        let (tx, rx) = bounded::<PlatformEvent>(256);
        let _focus_thread = focus::spawn_focus_monitor(Arc::clone(&self.focus_cache), tx.clone());
        let handle = hook::HookThreadHandle::start(tx, Arc::clone(&self.intercept_prtsc));
        if let Ok(mut guard) = self.hook_handle.lock() {
            *guard = Some(handle);
        }
        Ok(rx)
    }

    fn stop_hooks(&self) {
        if let Ok(mut guard) = self.hook_handle.lock() {
            if let Some(h) = guard.take() {
                h.stop();
            }
        }
    }

    fn current_focus(&self) -> FocusContext {
        self.focus_cache
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    fn current_layout(&self) -> Language {
        focus::query_foreground_layout()
    }

    fn set_layout(&self, layout: Language) -> PlatformResult<()> {
        injector::set_foreground_layout(layout)
    }

    fn replace_text(
        &self,
        backspace_count: usize,
        text: &str,
        switch_layout: Option<Language>,
    ) -> PlatformResult<()> {
        injector::replace_text(backspace_count, text, switch_layout)
    }

    fn insert_text(&self, text: &str) -> PlatformResult<()> {
        injector::insert_text(text)
    }

    fn transform_selection(&self, transform: SelectionTransform<'_>) -> PlatformResult<bool> {
        injector::transform_selection(transform)
    }

    fn read_selection_text(&self) -> PlatformResult<Option<String>> {
        injector::read_selection_text()
    }

    fn show_caret_badge(&self, layout: Language, duration_ms: u32) {
        badge::show_caret_badge(layout, duration_ms);
    }

    fn capture_virtual_screen(&self) -> PlatformResult<CapturedFrame> {
        capture::capture_virtual_screen()
    }

    fn copy_png_to_clipboard(&self, png_bytes: &[u8]) -> PlatformResult<()> {
        clipboard::set_png_image(png_bytes)
    }

    fn pin_screenshot(&self, png_bytes: Vec<u8>, screen_x: i32, screen_y: i32) -> PlatformResult<()> {
        pin_window::spawn_pinned_screenshot(png_bytes, screen_x, screen_y)
    }

    fn list_running_apps(&self) -> Vec<RunningAppInfo> {
        focus::list_running_apps()
    }

    fn set_autostart(&self, enabled: bool) -> PlatformResult<()> {
        autostart::set_autostart(enabled)
    }

    fn is_autostart_enabled(&self) -> bool {
        autostart::is_autostart_enabled()
    }

    fn set_intercept_print_screen(&self, intercept: bool) {
        self.intercept_prtsc.store(intercept, Ordering::Relaxed);
    }
}
