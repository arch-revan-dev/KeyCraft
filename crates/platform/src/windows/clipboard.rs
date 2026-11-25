//! Windows Clipboard utilities for UTF-16 text and lossless PNG / DIBV5 images.

use std::ffi::OsStr;
use std::io::Cursor;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::Graphics::Gdi::{BITMAPV5HEADER, BI_BITFIELDS};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

use crate::{PlatformError, PlatformResult};

const CF_UNICODETEXT: u32 = 13;
const CF_DIBV5: u32 = 17;
const LCS_SRGB: u32 = 0x7352_4742;

pub fn sequence_number() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

fn open_clipboard_retry() -> PlatformResult<()> {
    for _ in 0..8 {
        if unsafe { OpenClipboard(HWND::default()) }.is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
    Err(PlatformError::ClipboardError(
        "failed to open Windows clipboard".to_string(),
    ))
}

pub fn get_text() -> PlatformResult<Option<String>> {
    open_clipboard_retry()?;
    let res = unsafe {
        let handle = GetClipboardData(CF_UNICODETEXT);
        match handle {
            Ok(h) if !h.0.is_null() => {
                let ptr = GlobalLock(HGLOBAL(h.0));
                if ptr.is_null() {
                    Ok(None)
                } else {
                    let mut len = 0usize;
                    let wptr = ptr.cast::<u16>();
                    while *wptr.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(wptr, len);
                    let text = String::from_utf16_lossy(slice);
                    let _ = GlobalUnlock(HGLOBAL(h.0));
                    Ok(Some(text))
                }
            }
            _ => Ok(None),
        }
    };
    let _ = unsafe { CloseClipboard() };
    res
}

pub fn set_text(text: &str) -> PlatformResult<()> {
    let wide: Vec<u16> = OsStr::new(text).encode_wide().chain(Some(0)).collect();
    let byte_len = wide.len() * std::mem::size_of::<u16>();

    open_clipboard_retry()?;
    let res = unsafe {
        let _ = EmptyClipboard();
        let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len)
            .map_err(|e| PlatformError::ClipboardError(e.to_string()))?;
        let dst = GlobalLock(hmem);
        if dst.is_null() {
            return Err(PlatformError::ClipboardError("GlobalLock returned null".to_string()));
        }
        ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), dst.cast::<u8>(), byte_len);
        let _ = GlobalUnlock(hmem);
        SetClipboardData(CF_UNICODETEXT, HANDLE(hmem.0))
            .map_err(|e| PlatformError::ClipboardError(e.to_string()))?;
        Ok(())
    };
    let _ = unsafe { CloseClipboard() };
    res
}

/// Write BOTH registered `"PNG"` (preserves alpha & compression in modern apps)
/// AND `CF_DIBV5` (universal Win32 compatibility: Paint, Word, Telegram, Slack).
pub fn set_png_image(png_bytes: &[u8]) -> PlatformResult<()> {
    // Decode PNG to RGBA8 for CF_DIBV5
    let decoder = png::Decoder::new(Cursor::new(png_bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| PlatformError::ClipboardError(format!("PNG decode failed: {e}")))?;
    let mut img_buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut img_buf)
        .map_err(|e| PlatformError::ClipboardError(format!("PNG frame decode failed: {e}")))?;
    let width = info.width;
    let height = info.height;

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(&img_buf[..info.buffer_size()]),
        png::ColorType::Rgb => {
            for px in img_buf[..info.buffer_size()].as_chunks::<3>().0 {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        _ => {
            return Err(PlatformError::ClipboardError(
                "unsupported PNG color type".to_string(),
            ));
        }
    }

    // Build bottom-up BGRA DIBV5 payload
    let header_size = std::mem::size_of::<BITMAPV5HEADER>();
    let pixel_bytes = (width as usize) * (height as usize) * 4;
    let mut dib = vec![0u8; header_size + pixel_bytes];

    let header = BITMAPV5HEADER {
        bV5Size: header_size as u32,
        bV5Width: width as i32,
        bV5Height: -(height as i32), // top-down DIB
        bV5Planes: 1,
        bV5BitCount: 32,
        bV5Compression: BI_BITFIELDS,
        bV5SizeImage: pixel_bytes as u32,
        bV5RedMask: 0x00FF_0000,
        bV5GreenMask: 0x0000_FF00,
        bV5BlueMask: 0x0000_00FF,
        bV5AlphaMask: 0xFF00_0000,
        bV5CSType: LCS_SRGB,
        ..Default::default()
    };

    unsafe {
        ptr::copy_nonoverlapping(
            (&header as *const BITMAPV5HEADER).cast::<u8>(),
            dib.as_mut_ptr(),
            header_size,
        );
    }

    let dst_pixels = &mut dib[header_size..];
    for (src, dst) in rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(dst_pixels.as_chunks_mut::<4>().0.iter_mut())
    {
        dst[0] = src[2]; // B
        dst[1] = src[1]; // G
        dst[2] = src[0]; // R
        dst[3] = src[3]; // A
    }

    let png_fmt_name: Vec<u16> = OsStr::new("PNG").encode_wide().chain(Some(0)).collect();
    let cf_png = unsafe { RegisterClipboardFormatW(PCWSTR(png_fmt_name.as_ptr())) };

    open_clipboard_retry()?;
    let res = unsafe {
        let _ = EmptyClipboard();

        // 1. CF_DIBV5
        let hdib = GlobalAlloc(GMEM_MOVEABLE, dib.len())
            .map_err(|e| PlatformError::ClipboardError(e.to_string()))?;
        let pdib = GlobalLock(hdib);
        if !pdib.is_null() {
            ptr::copy_nonoverlapping(dib.as_ptr(), pdib.cast::<u8>(), dib.len());
            let _ = GlobalUnlock(hdib);
            SetClipboardData(CF_DIBV5, HANDLE(hdib.0))
                .map_err(|e| PlatformError::ClipboardError(e.to_string()))?;
        }

        // 2. Registered "PNG" format
        if cf_png != 0 {
            let hpng = GlobalAlloc(GMEM_MOVEABLE, png_bytes.len())
                .map_err(|e| PlatformError::ClipboardError(e.to_string()))?;
            let ppng = GlobalLock(hpng);
            if !ppng.is_null() {
                ptr::copy_nonoverlapping(png_bytes.as_ptr(), ppng.cast::<u8>(), png_bytes.len());
                let _ = GlobalUnlock(hpng);
                let _ = SetClipboardData(cf_png, HANDLE(hpng.0));
            }
        }
        Ok(())
    };
    let _ = unsafe { CloseClipboard() };
    res
}
