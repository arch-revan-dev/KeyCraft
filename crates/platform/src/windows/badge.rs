//! Pure Win32 layered popup badge (`RU` / `EN`) shown near the caret/cursor
//! for ~700 ms on layout switch with smooth alpha fade-out.
//!
//! Uses ZERO WebView2 instances (< 100 KB RAM).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicU64, Ordering};

use keycraft_core_engine::Language;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
    DIB_RGB_COLORS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, ShowWindow,
    UpdateLayeredWindow, SW_SHOWNOACTIVATE, ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::windows::focus::query_caret_or_cursor_pos;

const BADGE_W: u32 = 44;
const BADGE_H: u32 = 24;

static BADGE_GENERATION: AtomicU64 = AtomicU64::new(0);

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

/// Minimal crisp 5x7 pixel bitmap font for `R`, `U`, `E`, `N`.
fn glyph_rows(ch: char) -> [u8; 7] {
    match ch {
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        _ => [0; 7],
    }
}

fn render_badge_bgra(layout: Language) -> Vec<u8> {
    let mut pixmap = Pixmap::new(BADGE_W, BADGE_H).expect("badge pixmap");
    let w = BADGE_W as f32;
    let h = BADGE_H as f32;
    let r = 6.0f32;

    // Rounded pill background (#1C1C1F with subtle 1px border)
    let mut pb = PathBuilder::new();
    pb.move_to(r, 0.5);
    pb.line_to(w - r, 0.5);
    pb.quad_to(w - 0.5, 0.5, w - 0.5, r);
    pb.line_to(w - 0.5, h - r);
    pb.quad_to(w - 0.5, h - 0.5, w - r, h - 0.5);
    pb.line_to(r, h - 0.5);
    pb.quad_to(0.5, h - 0.5, 0.5, h - r);
    pb.line_to(0.5, r);
    pb.quad_to(0.5, 0.5, r, 0.5);
    pb.close();

    if let Some(path) = pb.finish() {
        let mut fill = Paint::default();
        fill.set_color(Color::from_rgba8(24, 24, 27, 238));
        fill.anti_alias = true;
        pixmap.fill_path(&path, &fill, FillRule::Winding, Transform::identity(), None);

        let mut border = Paint::default();
        border.set_color(Color::from_rgba8(255, 255, 255, 36));
        border.anti_alias = true;
        let stroke = Stroke {
            width: 1.0,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
    }

    // Crisp cobalt indicator dot + white 5x7 glyphs
    let mut dot_paint = Paint::default();
    dot_paint.set_color(match layout {
        Language::Ru => Color::from_rgba8(79, 139, 255, 255),
        Language::En => Color::from_rgba8(161, 161, 170, 255),
    });
    if let Some(dot) = PathBuilder::from_circle(9.5, h / 2.0, 2.5) {
        dot_paint.anti_alias = true;
        pixmap.fill_path(&dot, &dot_paint, FillRule::Winding, Transform::identity(), None);
    }

    let label = layout.label();
    let chars: Vec<char> = label.chars().collect();
    let start_x = 17usize;
    let start_y = 8usize;
    let data = pixmap.data_mut();
    for (ci, &ch) in chars.iter().enumerate() {
        let rows = glyph_rows(ch);
        let ox = start_x + ci * 8;
        for (ry, row) in rows.iter().enumerate() {
            for rx in 0..5usize {
                if (row >> (4 - rx)) & 1 == 1 {
                    let px = ox + rx;
                    let py = start_y + ry;
                    if px < BADGE_W as usize && py < BADGE_H as usize {
                        let idx = (py * BADGE_W as usize + px) * 4;
                        data[idx] = 244;
                        data[idx + 1] = 244;
                        data[idx + 2] = 245;
                        data[idx + 3] = 255;
                    }
                }
            }
        }
    }

    // tiny-skia stores premultiplied RGBA; Win32 UpdateLayeredWindow expects premultiplied BGRA
    let mut bgra = pixmap.take();
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    bgra
}

unsafe extern "system" fn badge_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Spawn a short-lived native Win32 layered window showing `RU`/`EN` at caret/cursor.
pub fn show_caret_badge(layout: Language, duration_ms: u32) {
    let gen = BADGE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let (cx, cy) = query_caret_or_cursor_pos();
    let pos_x = cx + 8;
    let pos_y = cy + 12;

    std::thread::Builder::new()
        .name("keycraft-caret-badge".to_string())
        .spawn(move || {
            let class_name = to_wide("KeyCraftCaretBadgeClass");
            let bgra = render_badge_bgra(layout);

            unsafe {
                let hinstance = GetModuleHandleW(None).unwrap_or_default();
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(badge_wnd_proc),
                    hInstance: hinstance.into(),
                    lpszClassName: PCWSTR(class_name.as_ptr()),
                    ..Default::default()
                };
                let _ = RegisterClassW(&wc);

                let Ok(hwnd) = CreateWindowExW(
                    WS_EX_LAYERED
                        | WS_EX_TRANSPARENT
                        | WS_EX_TOPMOST
                        | WS_EX_NOACTIVATE
                        | WS_EX_TOOLWINDOW,
                    PCWSTR(class_name.as_ptr()),
                    PCWSTR::null(),
                    WS_POPUP,
                    pos_x,
                    pos_y,
                    BADGE_W as i32,
                    BADGE_H as i32,
                    None,
                    None,
                    hinstance,
                    None,
                ) else {
                    return;
                };

                let hdc_screen = GetDC(HWND::default());
                let hdc_mem = CreateCompatibleDC(hdc_screen);
                let bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: BADGE_W as i32,
                        biHeight: -(BADGE_H as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
                let Ok(hbm) = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                else {
                    let _ = DeleteDC(hdc_mem);
                    ReleaseDC(HWND::default(), hdc_screen);
                    let _ = DestroyWindow(hwnd);
                    return;
                };
                if !bits.is_null() {
                    std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast::<u8>(), bgra.len());
                }
                let old = SelectObject(hdc_mem, hbm);

                let dst_pt = POINT { x: pos_x, y: pos_y };
                let size = SIZE {
                    cx: BADGE_W as i32,
                    cy: BADGE_H as i32,
                };
                let src_pt = POINT { x: 0, y: 0 };

                let mut blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                let _ = UpdateLayeredWindow(
                    hwnd,
                    hdc_screen,
                    Some(&dst_pt),
                    Some(&size),
                    hdc_mem,
                    Some(&src_pt),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                );
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

                let hold_ms = duration_ms.saturating_sub(150).max(200);
                let mut waited = 0u32;
                while waited < hold_ms && BADGE_GENERATION.load(Ordering::Relaxed) == gen {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                    waited += 25;
                }

                // Smooth 150ms fade-out
                for step in (0..10).rev() {
                    if BADGE_GENERATION.load(Ordering::Relaxed) != gen {
                        break;
                    }
                    blend.SourceConstantAlpha = ((step * 255) / 10) as u8;
                    let _ = UpdateLayeredWindow(
                        hwnd,
                        hdc_screen,
                        Some(&dst_pt),
                        Some(&size),
                        hdc_mem,
                        Some(&src_pt),
                        COLORREF(0),
                        Some(&blend),
                        ULW_ALPHA,
                    );
                    std::thread::sleep(std::time::Duration::from_millis(15));
                }

                SelectObject(hdc_mem, old);
                let _ = DeleteObject(hbm);
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(HWND::default(), hdc_screen);
                let _ = DestroyWindow(hwnd);
            }
        })
        .ok();
}
