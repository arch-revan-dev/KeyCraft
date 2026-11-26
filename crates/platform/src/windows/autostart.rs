//! Windows 11 Registry autostart manager (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

use crate::{PlatformError, PlatformResult};

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "KeyCraft";
const KEYBOARD_SUBKEY: &str = "Control Panel\\Keyboard";
const PRTSC_SNIPPING_VALUE: &str = "PrintScreenKeyForSnippingEnabled";

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

pub fn set_autostart(enabled: bool) -> PlatformResult<()> {
    let subkey = to_wide(RUN_SUBKEY);
    let val_name = to_wide(VALUE_NAME);

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if status.is_err() {
            return Err(PlatformError::OsError(format!(
                "RegOpenKeyExW failed: {status:?}"
            )));
        }

        let result = if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| PlatformError::OsError(e.to_string()))?;
            let cmd = format!("\"{}\"", exe.display());
            let cmd_wide = to_wide(&cmd);
            let bytes = std::slice::from_raw_parts(
                cmd_wide.as_ptr().cast::<u8>(),
                cmd_wide.len() * std::mem::size_of::<u16>(),
            );
            let st = RegSetValueExW(hkey, PCWSTR(val_name.as_ptr()), 0, REG_SZ, Some(bytes));
            if st.is_err() {
                Err(PlatformError::OsError(format!("RegSetValueExW failed: {st:?}")))
            } else {
                Ok(())
            }
        } else {
            let _ = RegDeleteValueW(hkey, PCWSTR(val_name.as_ptr()));
            Ok(())
        };

        let _ = RegCloseKey(hkey);
        result
    }
}

pub fn is_autostart_enabled() -> bool {
    let subkey = to_wide(RUN_SUBKEY);
    let val_name = to_wide(VALUE_NAME);
    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_QUERY_VALUE,
            &mut hkey,
        )
        .is_err()
        {
            return false;
        }
        let mut len = 0u32;
        let exists = RegQueryValueExW(
            hkey,
            PCWSTR(val_name.as_ptr()),
            None,
            None,
            None,
            Some(&mut len),
        )
        .is_ok();
        let _ = RegCloseKey(hkey);
        exists
    }
}

/// Check Windows 11 Accessibility setting `PrintScreenKeyForSnippingEnabled`.
/// On Windows 11 23H2/24H2, when this key is absent, Windows defaults it to `true` (Snipping Tool owns PrtSc).
pub fn is_windows_snipping_prtsc_enabled() -> bool {
    let subkey = to_wide(KEYBOARD_SUBKEY);
    let val_name = to_wide(PRTSC_SNIPPING_VALUE);
    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_QUERY_VALUE,
            &mut hkey,
        )
        .is_err()
        {
            return true;
        }
        let mut data = [0u8; 4];
        let mut len = data.len() as u32;
        let ok = RegQueryValueExW(
            hkey,
            PCWSTR(val_name.as_ptr()),
            None,
            None,
            Some(data.as_mut_ptr()),
            Some(&mut len),
        )
        .is_ok();
        let _ = RegCloseKey(hkey);
        if !ok || len < 4 {
            return true;
        }
        u32::from_le_bytes(data) != 0
    }
}

/// Set Windows 11 `HKCU\Control Panel\Keyboard\PrintScreenKeyForSnippingEnabled` (`0` = release PrtSc for KeyCraft,
/// `1` = restore Windows Snipping Tool binding) and notify Explorer via `WM_SETTINGCHANGE`.
pub fn set_windows_snipping_prtsc_enabled(enabled: bool) -> PlatformResult<()> {
    let subkey = to_wide(KEYBOARD_SUBKEY);
    let val_name = to_wide(PRTSC_SNIPPING_VALUE);
    let val: u32 = u32::from(enabled);
    let bytes = val.to_le_bytes();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let st = RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), &mut hkey);
        if st.is_err() {
            return Err(PlatformError::OsError(format!(
                "RegCreateKeyW(Keyboard) failed: {st:?}"
            )));
        }

        let res = RegSetValueExW(hkey, PCWSTR(val_name.as_ptr()), 0, REG_DWORD, Some(&bytes));
        let _ = RegCloseKey(hkey);
        if res.is_err() {
            return Err(PlatformError::OsError(format!(
                "RegSetValueExW(PrintScreenKeyForSnippingEnabled) failed: {res:?}"
            )));
        }

        let section = to_wide("Control Panel\\Keyboard");
        let mut _result = 0usize;
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(section.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            400,
            Some(&mut _result),
        );
        // Also broadcast a generic WM_SETTINGCHANGE for Shell accessibility listeners
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            200,
            Some(&mut _result),
        );
    }

    Ok(())
}
