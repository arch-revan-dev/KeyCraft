//! Windows 11 Registry autostart manager (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};

use crate::{PlatformError, PlatformResult};

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "KeyCraft";

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
