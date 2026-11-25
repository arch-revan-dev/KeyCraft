//! KeyCraft Core Engine — 100% offline, deterministic, platform-agnostic text engine.

pub mod buffer;
pub mod config;
pub mod dictionary;
pub mod engine;
pub mod guards;
pub mod history;
pub mod keymap;
pub mod lists_case;
pub mod snippets;
pub mod switcher;
pub mod typo;

pub use config::{
    AppRule, HotkeyConfig, KeyCraftConfig, Language, LayoutConfig, ListsCaseConfig, PinnedLayout,
    ScreenshotConfig, SnippetActivation, SnippetEntry, SystemConfig, ThemeMode, TypoConfig,
    UserWordEntry, WordSource,
};
pub use dictionary::Dictionary;
pub use engine::{ActiveAppState, EngineAction, EngineInput, KeyCraftEngine};
pub use guards::{check_word_guards, GuardVerdict, SkipReason};
pub use lists_case::{cycle_case, evaluate_smart_list_enter, SmartListAction};
