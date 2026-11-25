//! Local atomic JSON persistence for KeyCraft configuration (`%APPDATA%\KeyCraft\config.json`).
//! Never stores typed keystrokes or buffers.

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use keycraft_core_engine::KeyCraftConfig;

pub fn config_dir() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("org", "KeyCraft", "KeyCraft") {
        dirs.config_dir().to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn default_screenshots_dir() -> PathBuf {
    if let Some(user_dirs) = directories::UserDirs::new() {
        if let Some(pics) = user_dirs.picture_dir() {
            return pics.join("KeyCraft");
        }
    }
    config_dir().join("Screenshots")
}

pub fn load_config() -> KeyCraftConfig {
    let path = config_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return KeyCraftConfig::default();
    };
    serde_json::from_str::<KeyCraftConfig>(&raw).unwrap_or_default()
}

pub fn save_config(cfg: &KeyCraftConfig) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    let path = config_path();
    let tmp = dir.join("config.json.tmp");
    let json =
        serde_json::to_string_pretty(cfg).map_err(|e| format!("failed to serialize config: {e}"))?;
    fs::write(&tmp, json).map_err(|e| format!("failed to write temp config: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("failed to replace config file: {e}"))?;
    Ok(())
}
