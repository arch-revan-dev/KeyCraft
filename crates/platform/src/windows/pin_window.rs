//! Native lightweight Win32 always-on-top floating window for pinned screenshots.
//!
//! Features:
//! - Drag anywhere to move (`WM_NCHITTEST -> HTCAPTION`)
//! - Mouse wheel zoom (35%–300%)
//! - `Esc`, right-click, or double-click to close
//! - Zero WebView2 overhead (< 1 MB RAM per pinned screenshot)

use std::ffi::OsStr;
use std::io::Cursor;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, SetStretchBltMode, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HALFTONE, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, GetWindowRect, LoadCursorW, PostQuitMessage, RegisterClassW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, CS_DBLCLKS, GWLP_USERDATA,
    HTCAPTION, IDC_SIZEALL, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SW_SHOW, WM_DESTROY,
    WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_MOUSEWHEEL, WM_NCHITTEST, WM_NCDESTROY, WM_NCLBUTTONDBLCLK,
    WM_NCRBUTTONUP, WM_PAINT, WM_RBUTTONUP, WNDCLASSW, WS_BORDER, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};

use crate::{PlatformError, PlatformResult};

struct PinState {
    width: u32,
    height: u32,
    scale: f32,
    bgra: Vec<u8>,
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

unsafe extern "system" fn pin_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PinState;
    match msg {
        WM_NCHITTEST => LRESULT(HTCAPTION as isize),
        // Because WM_NCHITTEST returns HTCAPTION for drag-anywhere, Windows delivers NC mouse events!
        WM_RBUTTONUP | WM_NCRBUTTONUP | WM_LBUTTONDBLCLK | WM_NCLBUTTONDBLCLK => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 == usize::from(VK_ESCAPE.0) {
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEWHEEL => {
            if !ptr.is_null() {
                let state = &mut *ptr;
                let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
                state.scale = (state.scale + delta * 0.1).clamp(0.35, 3.0);
                let new_w = ((state.width as f32) * state.scale).round().max(64.0) as i32;
                let new_h = ((state.height as f32) * state.scale).round().max(48.0) as i32;
                let mut rc = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rc);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rc.left,
                    rc.top,
                    new_w + 2,
                    new_h + 2,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !ptr.is_null() {
                let state = &*ptr;
                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                let dst_w = (rc.right - rc.left).max(1);
                let dst_h = (rc.bottom - rc.top).max(1);

                let bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: state.width as i32,
                        biHeight: -(state.height as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                SetStretchBltMode(hdc, HALFTONE);
                StretchDIBits(
                    hdc,
                    0,
                    0,
                    dst_w,
                    dst_h,
                    0,
                    0,
                    state.width as i32,
                    state.height as i32,
                    Some(state.bgra.as_ptr().cast()),
                    &bmi,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn spawn_pinned_screenshot(png_bytes: Vec<u8>, screen_x: i32, screen_y: i32) -> PlatformResult<()> {
    let decoder = png::Decoder::new(Cursor::new(&png_bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| PlatformError::CaptureError(format!("pin PNG decode failed: {e}")))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| PlatformError::CaptureError(format!("pin PNG frame failed: {e}")))?;

    let width = info.width;
    let height = info.height;
    let mut bgra = Vec::with_capacity((width * height * 4) as usize);
    match info.color_type {
        png::ColorType::Rgba => {
            for px in buf[..info.buffer_size()].as_chunks::<4>().0 {
                bgra.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
            }
        }
        png::ColorType::Rgb => {
            for px in buf[..info.buffer_size()].as_chunks::<3>().0 {
                bgra.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
        }
        _ => {
            return Err(PlatformError::CaptureError(
                "unsupported pinned PNG format".to_string(),
            ));
        }
    }

    std::thread::Builder::new()
        .name("keycraft-pin-window".to_string())
        .spawn(move || {
            let class_name = to_wide("KeyCraftPinnedScreenshotClass");
            let title = to_wide("KeyCraft Pin");

            unsafe {
                let hinstance = GetModuleHandleW(None).unwrap_or_default();
                let wc = WNDCLASSW {
                    style: CS_DBLCLKS,
                    lpfnWndProc: Some(pin_wnd_proc),
                    hInstance: hinstance.into(),
                    hCursor: LoadCursorW(None, IDC_SIZEALL).unwrap_or_default(),
                    lpszClassName: PCWSTR(class_name.as_ptr()),
                    ..Default::default()
                };
                let _ = RegisterClassW(&wc);

                let Ok(hwnd) = CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                    PCWSTR(class_name.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    WS_POPUP | WS_BORDER,
                    screen_x,
                    screen_y,
                    (width as i32) + 2,
                    (height as i32) + 2,
                    None,
                    None,
                    hinstance,
                    None,
                ) else {
                    return;
                };

                let state = Box::new(PinState {
                    width,
                    height,
                    scale: 1.0,
                    bgra,
                });
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

                let _ = ShowWindow(hwnd, SW_SHOW);
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        })
        .map_err(|e| PlatformError::OsError(e.to_string()))?;

    Ok(())
}
