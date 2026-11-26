//! Configuration, user dictionary, snippet, and per-app rule schemas for KeyCraft.
//! All structures serialize cleanly to/from local JSON (zero cloud sync).

use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Ru,
    En,
}

impl Language {
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Ru => Self::En,
            Self::En => Self::Ru,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ru => "RU",
            Self::En => "EN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PinnedLayout {
    #[default]
    Keep,
    En,
    Ru,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SnippetActivation {
    #[default]
    Space,
    Tab,
    Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WordSource {
    #[default]
    Manual,
    AutoLearned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub auto_switch: bool,
    pub fix_capslock: bool,
    pub single_symbol_swap: bool,
    #[serde(default)]
    pub undo_on_backspace: bool,
    pub auto_learn_enabled: bool,
    pub auto_learn_threshold: u8,
    pub switch_system_layout: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            auto_switch: true,
            fix_capslock: true,
            single_symbol_swap: true,
            undo_on_backspace: false,
            auto_learn_enabled: true,
            auto_learn_threshold: 3,
            switch_system_layout: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypoConfig {
    pub enabled: bool,
    pub min_word_length: usize,
}

impl Default for TypoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_word_length: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListsCaseConfig {
    pub auto_capitalize_sentences: bool,
    pub smart_lists: bool,
}

impl Default for ListsCaseConfig {
    fn default() -> Self {
        Self {
            auto_capitalize_sentences: true,
            smart_lists: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetEntry {
    pub id: String,
    pub triggers: Vec<String>,
    pub replacement: String,
    pub activation: SnippetActivation,
    pub any_layout: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserWordEntry {
    pub word: String,
    pub source: WordSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRule {
    pub process_name: String,
    pub display_name: String,
    pub disable_auto_switch: bool,
    pub disable_typos: bool,
    pub disable_snippets: bool,
    pub disable_lists: bool,
    pub disable_all: bool,
    pub pinned_layout: PinnedLayout,
}

impl AppRule {
    #[must_use]
    pub fn disabled_all(process_name: &str, display_name: &str, pinned_layout: PinnedLayout) -> Self {
        Self {
            process_name: process_name.to_string(),
            display_name: display_name.to_string(),
            disable_auto_switch: true,
            disable_typos: true,
            disable_snippets: true,
            disable_lists: true,
            disable_all: true,
            pinned_layout,
        }
    }

    #[must_use]
    pub fn ide_preset(process_name: &str, display_name: &str) -> Self {
        Self {
            process_name: process_name.to_string(),
            display_name: display_name.to_string(),
            disable_auto_switch: true,
            disable_typos: true,
            disable_snippets: false,
            disable_lists: true,
            disable_all: false,
            pinned_layout: PinnedLayout::En,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub convert_selection_or_word: String,
    pub undo_last_action: String,
    pub swap_last_symbol: String,
    pub cycle_case: String,
    pub add_to_dictionary: String,
    pub screenshot_region: String,
    pub screenshot_window: String,
    pub screenshot_fullscreen: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            convert_selection_or_word: "Pause".to_string(),
            undo_last_action: "Escape".to_string(),
            swap_last_symbol: "RAlt".to_string(),
            cycle_case: "Alt+Shift+U".to_string(),
            add_to_dictionary: "Ctrl+Alt+D".to_string(),
            screenshot_region: "PrintScreen".to_string(),
            screenshot_window: "Alt+PrintScreen".to_string(),
            screenshot_fullscreen: "Ctrl+PrintScreen".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenshotConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub save_dir: String,
    pub filename_pattern: String,
    pub show_magnifier: bool,
    pub copy_after_save: bool,
    pub default_color: String,
    pub default_stroke_width: u8,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            save_dir: String::new(),
            filename_pattern: "KeyCraft_%Y-%m-%d_%H-%M-%S".to_string(),
            show_magnifier: true,
            copy_after_save: true,
            default_color: "#DC2626".to_string(),
            default_stroke_width: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemConfig {
    pub theme: ThemeMode,
    pub autostart: bool,
    pub show_caret_badge: bool,
    pub caret_badge_duration_ms: u32,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            autostart: false,
            show_caret_badge: true,
            caret_badge_duration_ms: 700,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyCraftConfig {
    pub version: u32,
    pub enabled: bool,
    pub layout: LayoutConfig,
    pub typos: TypoConfig,
    pub lists_case: ListsCaseConfig,
    pub snippets: Vec<SnippetEntry>,
    pub dictionary: Vec<UserWordEntry>,
    pub app_rules: Vec<AppRule>,
    pub hotkeys: HotkeyConfig,
    pub screenshot: ScreenshotConfig,
    pub system: SystemConfig,
}

impl Default for KeyCraftConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            enabled: true,
            layout: LayoutConfig::default(),
            typos: TypoConfig::default(),
            lists_case: ListsCaseConfig::default(),
            snippets: default_snippets(),
            dictionary: Vec::new(),
            app_rules: default_app_rules(),
            hotkeys: HotkeyConfig::default(),
            screenshot: ScreenshotConfig::default(),
            system: SystemConfig::default(),
        }
    }
}

impl KeyCraftConfig {
    #[must_use]
    pub fn find_app_rule(&self, process_name: &str) -> Option<&AppRule> {
        let needle = process_name.trim().to_ascii_lowercase();
        self.app_rules
            .iter()
            .find(|r| r.process_name.to_ascii_lowercase() == needle)
    }
}

#[must_use]
pub fn default_snippets() -> Vec<SnippetEntry> {
    vec![
        SnippetEntry {
            id: "rub".to_string(),
            triggers: vec!["р-".to_string(), "Р-".to_string()],
            replacement: "₽".to_string(),
            activation: SnippetActivation::Space,
            any_layout: true,
            enabled: true,
        },
        SnippetEntry {
            id: "mdash".to_string(),
            triggers: vec!["---".to_string()],
            replacement: "—".to_string(),
            activation: SnippetActivation::Space,
            any_layout: true,
            enabled: true,
        },
        SnippetEntry {
            id: "laquo".to_string(),
            triggers: vec!["<<".to_string()],
            replacement: "«".to_string(),
            activation: SnippetActivation::Instant,
            any_layout: true,
            enabled: true,
        },
        SnippetEntry {
            id: "raquo".to_string(),
            triggers: vec![">>".to_string()],
            replacement: "»".to_string(),
            activation: SnippetActivation::Instant,
            any_layout: true,
            enabled: true,
        },
        SnippetEntry {
            id: "plusminus".to_string(),
            triggers: vec!["+-".to_string()],
            replacement: "±".to_string(),
            activation: SnippetActivation::Space,
            any_layout: true,
            enabled: true,
        },
        SnippetEntry {
            id: "arrow".to_string(),
            triggers: vec!["->".to_string()],
            replacement: "→".to_string(),
            activation: SnippetActivation::Space,
            any_layout: true,
            enabled: true,
        },
        SnippetEntry {
            id: "copyright".to_string(),
            triggers: vec!["(c)".to_string(), "(C)".to_string()],
            replacement: "©".to_string(),
            activation: SnippetActivation::Instant,
            any_layout: true,
            enabled: true,
        },
    ]
}

#[must_use]
pub fn default_app_rules() -> Vec<AppRule> {
    vec![
        AppRule::disabled_all("WindowsTerminal.exe", "Windows Terminal", PinnedLayout::En),
        AppRule::ide_preset("Code.exe", "Visual Studio Code"),
        AppRule::ide_preset("idea64.exe", "IntelliJ IDEA"),
        AppRule::disabled_all("pwsh.exe", "PowerShell", PinnedLayout::En),
        AppRule::disabled_all("cmd.exe", "Командная строка", PinnedLayout::En),
        AppRule::disabled_all("Alacritty.exe", "Alacritty", PinnedLayout::En),
        AppRule::disabled_all("wezterm-gui.exe", "WezTerm", PinnedLayout::En),
        AppRule::disabled_all("iTerm2", "iTerm2", PinnedLayout::En),
    ]
}
