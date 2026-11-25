//! Offline DPI-aware screen capture (DXGI Desktop Duplication + seamless GDI BitBlt fallback)
//! and DWM extended window-frame enumeration for active-window / hover-snap screenshots.

use std::io::Cursor;
use std::ptr;

use windows::core::Interface;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, IDXGIOutput1, IDXGIResource, DXGI_OUTDUPL_FRAME_INFO,
};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetSystemMetrics, GetWindowLongW, GetWindowRect,
    IsIconic, IsWindowVisible, GWL_EXSTYLE, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, WS_EX_TOOLWINDOW,
};

use crate::{CapturedFrame, PlatformError, PlatformResult, ScreenRect};

fn encode_rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> PlatformResult<Vec<u8>> {
    let mut out = Vec::with_capacity((width * height) as usize);
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut out), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder
            .write_header()
            .map_err(|e| PlatformError::CaptureError(format!("PNG header error: {e}")))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| PlatformError::CaptureError(format!("PNG encode error: {e}")))?;
    }
    Ok(out)
}

fn extended_window_rect(hwnd: HWND) -> Option<ScreenRect> {
    unsafe {
        let mut rect = RECT::default();
        let dwm_ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut rect as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
        .is_ok();
        if !dwm_ok && GetWindowRect(hwnd, &mut rect).is_err() {
            return None;
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w < 24 || h < 24 {
            return None;
        }
        Some(ScreenRect {
            x: rect.left,
            y: rect.top,
            width: w as u32,
            height: h as u32,
        })
    }
}

fn enumerate_visible_window_rects() -> Vec<ScreenRect> {
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if (ex & WS_EX_TOOLWINDOW.0) != 0 {
            return BOOL(1);
        }
        if let Some(r) = extended_window_rect(hwnd) {
            let list = &mut *(lparam.0 as *mut Vec<ScreenRect>);
            if list.len() < 48 {
                list.push(r);
            }
        }
        BOOL(1)
    }

    let mut rects = Vec::with_capacity(24);
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM((&mut rects as *mut Vec<ScreenRect>) as isize));
    }
    rects
}

/// Primary fast path: DXGI Desktop Duplication for single-monitor setups (zero GDI flicker).
fn try_capture_dxgi_primary(expected_w: u32, expected_h: u32) -> Option<Vec<u8>> {
    unsafe {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
        .ok()?;
        let device = device?;
        let context = context?;
        let dxgi_device: IDXGIDevice = device.cast().ok()?;
        let adapter = dxgi_device.GetAdapter().ok()?;
        let output = adapter.EnumOutputs(0).ok()?;
        let output1: IDXGIOutput1 = output.cast().ok()?;
        let dupl = output1.DuplicateOutput(&device).ok()?;

        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource: Option<IDXGIResource> = None;
        // Try up to 2 short polls so DXGI delivers a fresh frame
        let mut acquired = false;
        for _ in 0..2 {
            if dupl.AcquireNextFrame(120, &mut frame_info, &mut resource).is_ok() {
                acquired = true;
                break;
            }
        }
        if !acquired {
            return None;
        }
        let res = resource?;
        let tex: ID3D11Texture2D = res.cast().ok()?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        tex.GetDesc(&mut desc);
        if desc.Width != expected_w || desc.Height != expected_h {
            let _ = dupl.ReleaseFrame();
            return None;
        }

        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: desc.Width,
            Height: desc.Height,
            MipLevels: 1,
            ArraySize: 1,
            Format: desc.Format,
            SampleDesc: desc.SampleDesc,
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };
        let mut staging: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&staging_desc, None, Some(&mut staging)).ok()?;
        let staging = staging?;
        context.CopyResource(&staging, &tex);

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).ok()?;

        let w = desc.Width as usize;
        let h = desc.Height as usize;
        let pitch = mapped.RowPitch as usize;
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            let src_row = std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(y * pitch), w * 4);
            let dst_row = &mut rgba[y * w * 4..(y + 1) * w * 4];
            for (src, dst) in src_row
                .as_chunks::<4>()
                .0
                .iter()
                .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
            {
                dst[0] = src[2]; // R
                dst[1] = src[1]; // G
                dst[2] = src[0]; // B
                dst[3] = 255;    // A
            }
        }

        context.Unmap(&staging, 0);
        let _ = dupl.ReleaseFrame();
        Some(rgba)
    }
}

/// Bulletproof virtual-desktop GDI BitBlt capture supporting multi-monitor, mixed DPI, and RDP.
fn capture_gdi_virtual(x: i32, y: i32, width: u32, height: u32) -> PlatformResult<Vec<u8>> {
    unsafe {
        let hdc_screen = GetDC(HWND::default());
        if hdc_screen.is_invalid() {
            return Err(PlatformError::CaptureError("GetDC failed".to_string()));
        }
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, width as i32, height as i32);
        let old = SelectObject(hdc_mem, hbm);

        let blit_ok = BitBlt(
            hdc_mem,
            0,
            0,
            width as i32,
            height as i32,
            hdc_screen,
            x,
            y,
            SRCCOPY,
        )
        .is_ok();

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
        let lines = if blit_ok {
            GetDIBits(
                hdc_mem,
                hbm,
                0,
                height,
                Some(bgra.as_mut_ptr().cast()),
                &mut bmi,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };

        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(HWND::default(), hdc_screen);

        if !blit_ok || lines == 0 {
            return Err(PlatformError::CaptureError("GDI BitBlt/GetDIBits failed".to_string()));
        }

        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2); // BGRA -> RGBA
            px[3] = 255;
        }
        Ok(bgra)
    }
}

pub fn capture_virtual_screen() -> PlatformResult<CapturedFrame> {
    let origin_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let origin_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) }.max(1) as u32;
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) }.max(1) as u32;

    let fg = unsafe { GetForegroundWindow() };
    let active_window_rect = if fg.0.is_null() {
        None
    } else {
        extended_window_rect(fg)
    };
    let visible_windows = enumerate_visible_window_rects();

    // Try DXGI Desktop Duplication first when single-monitor matches virtual bounds, else GDI BitBlt
    let rgba = if origin_x == 0 && origin_y == 0 {
        match try_capture_dxgi_primary(width, height) {
            Some(buf) => buf,
            None => capture_gdi_virtual(origin_x, origin_y, width, height)?,
        }
    } else {
        capture_gdi_virtual(origin_x, origin_y, width, height)?
    };

    let png_bytes = encode_rgba_to_png(width, height, &rgba)?;

    Ok(CapturedFrame {
        origin_x,
        origin_y,
        width,
        height,
        png_bytes,
        active_window_rect,
        visible_windows,
    })
}

#[allow(dead_code)]
fn _null_ptr() -> *const u8 {
    ptr::null()
}
