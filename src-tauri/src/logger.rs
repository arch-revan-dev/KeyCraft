//! Zero-Keylogger diagnostic & crash logger (`%APPDATA%\KeyCraft\KeyCraft\logs\keycraft.log`).
//!
//! PRIVACY INVARIANT: NEVER log keystrokes, typed words, window titles, or clipboard contents.
//! Logs strictly lifecycle events, subsystem errors, and panic hooks, capped at 128 KB.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;

use crate::store;

const MAX_LOG_BYTES: u64 = 128 * 1024;

static LOG_LOCK: Mutex<()> = Mutex::new(());

pub fn log_file_path() -> PathBuf {
    store::config_dir().join("logs").join("keycraft.log")
}

pub fn log_line(level: &str, message: &str) {
    let Ok(_guard) = LOG_LOCK.lock() else {
        return;
    };
    let path = log_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Rotate if log exceeds 128 KB so disk usage stays tiny.
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > MAX_LOG_BYTES {
            let prev = path.with_extension("log.prev");
            let _ = fs::remove_file(&prev);
            let _ = fs::rename(&path, &prev);
        }
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{ts}] [{level}] {message}");
    }
}

pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        log_line("PANIC", &format!("at {location}: {payload}"));
        default_hook(info);
    }));
}
