//! Unified deterministic event processing pipeline for KeyCraft (`< 2 ms` per event).
//! Consumed identically by the Windows 11 background daemon and the Settings UI Sandbox.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::buffer::InputBuffer;
use crate::config::{KeyCraftConfig, Language, SnippetActivation, UserWordEntry, WordSource};
use crate::dictionary::Dictionary;
use crate::history::{CorrectionHistory, CorrectionKind, UndoRecord};
use crate::keymap::{convert_layout_auto, swap_special_symbol};
use crate::lists_case::{cycle_case, evaluate_smart_list_enter, SmartListAction};
use crate::snippets::SnippetMatcher;
use crate::switcher::{capitalize_first, evaluate_layout_switch, SwitchKind};
use crate::typo::fix_typo;

/// Logical input events fed into the cross-platform engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineInput {
    /// Printable character typed by the user.
    Char { ch: char, layout: Language },
    /// Space key pressed.
    Space { layout: Language },
    /// Tab key pressed.
    Tab { layout: Language },
    /// Enter / Return key pressed.
    Enter { layout: Language },
    /// Backspace key pressed.
    Backspace,
    /// Mouse click, focus change, arrow key, or modifier shortcut that invalidates word context.
    ResetContext,
    /// User pressed Undo hotkey (e.g. `Escape` or single-tap `LAlt`).
    UndoLast,
    /// User pressed single special-symbol swap key (e.g. single-tap `RAlt`).
    SwapLastSymbol { layout: Language },
    /// User pressed manual convert hotkey on the last typed word (when nothing is selected).
    ManualConvertLastWord,
    /// User pressed cycle-case hotkey on the last typed word (when nothing is selected).
    ManualCycleCaseLastWord,
}

/// Active application context evaluated per event.
#[derive(Debug, Clone, Default)]
pub struct ActiveAppState {
    pub process_name: String,
    pub is_password_field: bool,
}

/// Deterministic output actions for the platform injector or UI sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineAction {
    NoOp,
    /// Delete `backspace_count` characters and type `text` (optionally switching OS layout).
    ReplaceText {
        backspace_count: usize,
        text: String,
        switch_layout: Option<Language>,
        reason: String,
        elapsed_us: u64,
        auto_learned_word: Option<String>,
    },
    /// Insert text without deleting prior characters (e.g. smart list continuation after Enter).
    InsertText {
        text: String,
        reason: String,
        elapsed_us: u64,
    },
}

pub struct KeyCraftEngine {
    config: KeyCraftConfig,
    dict: Dictionary,
    snippets: SnippetMatcher,
    buffer: InputBuffer,
    history: CorrectionHistory,
    current_layout: Language,
    just_replaced_last_word: bool,
}

impl KeyCraftEngine {
    #[must_use]
    pub fn new(config: KeyCraftConfig) -> Self {
        let dict = Dictionary::load_embedded(&config.dictionary);
        let snippets = SnippetMatcher::compile(&config.snippets);
        Self {
            config,
            dict,
            snippets,
            buffer: InputBuffer::new(),
            history: CorrectionHistory::new(),
            current_layout: Language::En,
            just_replaced_last_word: false,
        }
    }

    #[must_use]
    pub fn config(&self) -> &KeyCraftConfig {
        &self.config
    }

    #[must_use]
    pub fn dictionary(&self) -> &Dictionary {
        &self.dict
    }

    pub fn update_config(&mut self, config: KeyCraftConfig) {
        self.dict.sync_user_words(&config.dictionary);
        self.snippets = SnippetMatcher::compile(&config.snippets);
        self.config = config;
    }

    pub fn add_user_word(&mut self, word: &str, source: WordSource) -> bool {
        let norm = word.trim().to_lowercase();
        if norm.is_empty() {
            return false;
        }
        if self.dict.add_user_word(&norm) {
            self.config.dictionary.push(UserWordEntry { word: norm, source });
            return true;
        }
        false
    }

    pub fn reset_buffer(&mut self) {
        self.buffer.clear_all();
        self.history.clear_last();
        self.just_replaced_last_word = false;
    }

    /// Convert arbitrary selected text manually (preserving words/punctuation).
    #[must_use]
    pub fn convert_selected_text(&self, text: &str) -> (String, Language) {
        convert_layout_auto(text)
    }

    /// Cycle case of arbitrary selected text (`привет мир` → `Привет мир` → `ПРИВЕТ МИР` → `Привет Мир`).
    #[must_use]
    pub fn cycle_selected_case(&self, text: &str) -> String {
        cycle_case(text)
    }

    /// Process a single input event under the active app context.
    pub fn process(&mut self, input: EngineInput, app: &ActiveAppState) -> EngineAction {
        let started = Instant::now();

        // Zero-Keylogger invariant: password fields IMMEDIATELY zeroize and bypass everything.
        if app.is_password_field || !self.config.enabled {
            self.buffer.clear_all();
            self.history.clear_last();
            return EngineAction::NoOp;
        }

        let rule = self.config.find_app_rule(&app.process_name).cloned();
        if let Some(ref r) = rule {
            if r.disable_all {
                self.buffer.clear_all();
                return EngineAction::NoOp;
            }
        }

        let allow_layout = rule.as_ref().is_none_or(|r| !r.disable_auto_switch);
        let allow_typos = rule.as_ref().is_none_or(|r| !r.disable_typos);
        let allow_snippets = rule.as_ref().is_none_or(|r| !r.disable_snippets);
        let allow_lists = rule.as_ref().is_none_or(|r| !r.disable_lists);

        match input {
            EngineInput::ResetContext => {
                self.buffer.clear_all();
                self.just_replaced_last_word = false;
                EngineAction::NoOp
            }
            EngineInput::Backspace => {
                self.buffer.pop_char();
                self.just_replaced_last_word = false;
                EngineAction::NoOp
            }
            EngineInput::UndoLast => self.handle_undo(started),
            EngineInput::SwapLastSymbol { layout } => {
                self.current_layout = layout;
                self.handle_symbol_swap(layout, started)
            }
            EngineInput::ManualConvertLastWord => self.handle_manual_convert_last_word(started),
            EngineInput::ManualCycleCaseLastWord => self.handle_manual_cycle_case(started),
            EngineInput::Char { ch, layout } => {
                self.current_layout = layout;
                self.just_replaced_last_word = false;
                self.history.clear_last();
                self.buffer.push_word_char(ch);

                // Check Instant snippets on suffix of current word buffer.
                if allow_snippets {
                    if let Some(m) = self.snippets.match_instant(self.buffer.current_word()) {
                        let repl = m.replacement.clone();
                        self.buffer.clear_word_only();
                        self.buffer.set_last_char(repl.chars().last());
                        self.history.record(UndoRecord {
                            original_word: String::new(),
                            replaced_word: repl.clone(),
                            trailing_delimiter: None,
                            restore_layout: None,
                            kind: CorrectionKind::Snippet,
                        });
                        return EngineAction::ReplaceText {
                            backspace_count: m.trigger_char_len,
                            text: repl,
                            switch_layout: None,
                            reason: format!("сниппет ({})", m.snippet_id),
                            elapsed_us: started.elapsed().as_micros() as u64,
                            auto_learned_word: None,
                        };
                    }
                }

                EngineAction::NoOp
            }
            EngineInput::Space { layout } => {
                self.current_layout = layout;
                self.handle_delimiter(
                    ' ',
                    SnippetActivation::Space,
                    layout,
                    allow_layout,
                    allow_typos,
                    allow_snippets,
                    started,
                )
            }
            EngineInput::Tab { layout } => {
                self.current_layout = layout;
                self.handle_delimiter(
                    '\t',
                    SnippetActivation::Tab,
                    layout,
                    allow_layout,
                    allow_typos,
                    allow_snippets,
                    started,
                )
            }
            EngineInput::Enter { layout } => {
                self.current_layout = layout;
                let line_snapshot = self.buffer.current_line().to_string();

                // 1. Smart list continuation / exit on Enter (takes priority when line is a list item and word is empty or normal).
                if allow_lists && self.config.lists_case.smart_lists {
                    if let Some(list_action) = evaluate_smart_list_enter(&line_snapshot) {
                        self.buffer.clear_word_only();
                        self.buffer.clear_line();
                        self.buffer.set_sentence_start(true);
                        match list_action {
                            SmartListAction::ExitEmptyItem { erase_chars } => {
                                // Erase the newline that Enter just inserted (+1) AND the empty marker.
                                return EngineAction::ReplaceText {
                                    backspace_count: erase_chars + 1,
                                    text: String::new(),
                                    switch_layout: None,
                                    reason: "выход из списка".to_string(),
                                    elapsed_us: started.elapsed().as_micros() as u64,
                                    auto_learned_word: None,
                                };
                            }
                            SmartListAction::InsertNextMarker(marker) => {
                                for c in marker.chars() {
                                    self.buffer.push_line_char(c);
                                }
                                return EngineAction::InsertText {
                                    text: marker,
                                    reason: "продолжение списка".to_string(),
                                    elapsed_us: started.elapsed().as_micros() as u64,
                                };
                            }
                        }
                    }
                }

                let action = self.handle_delimiter(
                    '\n',
                    SnippetActivation::Space,
                    layout,
                    allow_layout,
                    allow_typos,
                    false,
                    started,
                );
                self.buffer.clear_line();
                self.buffer.set_sentence_start(true);
                action
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_delimiter(
        &mut self,
        delimiter: char,
        snippet_activation: SnippetActivation,
        layout: Language,
        allow_layout: bool,
        allow_typos: bool,
        allow_snippets: bool,
        started: Instant,
    ) -> EngineAction {
        let word = self.buffer.commit_word();
        self.buffer.push_line_char(delimiter);
        self.buffer.set_last_char(Some(delimiter));

        if word.is_empty() {
            return EngineAction::NoOp;
        }

        let was_sentence_start = self.buffer.is_sentence_start();
        let ends_sentence = word.ends_with('.') || word.ends_with('!') || word.ends_with('?');
        self.buffer.set_sentence_start(ends_sentence);

        // 1. Snippets on Space / Tab
        if allow_snippets {
            if let Some(m) = self.snippets.match_on_delimiter(&word, snippet_activation) {
                let mut replacement = m.replacement.clone();
                if delimiter == ' ' {
                    replacement.push(' ');
                }
                self.buffer.update_last_completed_word(&m.replacement);
                self.just_replaced_last_word = true;
                self.history.record(UndoRecord {
                    original_word: word,
                    replaced_word: m.replacement,
                    trailing_delimiter: if delimiter == ' ' { Some(' ') } else { None },
                    restore_layout: None,
                    kind: CorrectionKind::Snippet,
                });
                return EngineAction::ReplaceText {
                    backspace_count: m.trigger_char_len + 1,
                    text: replacement,
                    switch_layout: None,
                    reason: format!("сниппет ({})", m.snippet_id),
                    elapsed_us: started.elapsed().as_micros() as u64,
                    auto_learned_word: None,
                };
            }
        }

        // 2. Auto Layout Switch & Accidental CapsLock
        if allow_layout {
            if let Some(decision) = evaluate_layout_switch(&word, &self.dict, &self.config.layout) {
                let (target_layout, kind, reason) = match decision.kind {
                    SwitchKind::LayoutSwitch { target_layout } => (
                        Some(target_layout),
                        CorrectionKind::LayoutSwitch,
                        format!("раскладка → {}", target_layout.label()),
                    ),
                    SwitchKind::CapsLockFix => (
                        None,
                        CorrectionKind::CapsLockFix,
                        "исправление CapsLock".to_string(),
                    ),
                    SwitchKind::LayoutAndCapsLockFix { target_layout } => (
                        Some(target_layout),
                        CorrectionKind::LayoutSwitch,
                        format!("раскладка → {} + CapsLock", target_layout.label()),
                    ),
                };

                let mut final_word = decision.replacement;
                if was_sentence_start
                    && self.config.lists_case.auto_capitalize_sentences
                    && final_word.chars().next().is_some_and(char::is_lowercase)
                {
                    final_word = capitalize_first(&final_word);
                }

                let switch_os = if self.config.layout.switch_system_layout {
                    target_layout
                } else {
                    None
                };

                let mut out_text = final_word.clone();
                out_text.push(delimiter);

                self.buffer.update_last_completed_word(&final_word);
                self.just_replaced_last_word = true;
                self.history.record(UndoRecord {
                    original_word: word.clone(),
                    replaced_word: final_word,
                    trailing_delimiter: Some(delimiter),
                    restore_layout: target_layout.map(Language::opposite),
                    kind,
                });

                return EngineAction::ReplaceText {
                    backspace_count: word.chars().count() + 1,
                    text: out_text,
                    switch_layout: switch_os,
                    reason,
                    elapsed_us: started.elapsed().as_micros() as u64,
                    auto_learned_word: None,
                };
            }
        }

        // 3. Offline Deterministic Typo Correction (DL = 1)
        if allow_typos {
            if let Some(mut fixed) = fix_typo(&word, &self.dict, &self.config.typos) {
                if was_sentence_start
                    && self.config.lists_case.auto_capitalize_sentences
                    && fixed.chars().next().is_some_and(char::is_lowercase)
                {
                    fixed = capitalize_first(&fixed);
                }
                let mut out_text = fixed.clone();
                out_text.push(delimiter);

                self.buffer.update_last_completed_word(&fixed);
                self.just_replaced_last_word = true;
                self.history.record(UndoRecord {
                    original_word: word.clone(),
                    replaced_word: fixed,
                    trailing_delimiter: Some(delimiter),
                    restore_layout: None,
                    kind: CorrectionKind::TypoFix,
                });

                return EngineAction::ReplaceText {
                    backspace_count: word.chars().count() + 1,
                    text: out_text,
                    switch_layout: None,
                    reason: "исправление опечатки".to_string(),
                    elapsed_us: started.elapsed().as_micros() as u64,
                    auto_learned_word: None,
                };
            }
        }

        // 4. Optional Sentence Auto-Capitalization
        if self.config.lists_case.auto_capitalize_sentences
            && was_sentence_start
            && word.chars().next().is_some_and(char::is_lowercase)
            && crate::guards::check_word_guards(&word) == crate::guards::GuardVerdict::Allow
        {
            let cap = capitalize_first(&word);
            if cap != word {
                let mut out_text = cap.clone();
                out_text.push(delimiter);
                self.buffer.update_last_completed_word(&cap);
                self.just_replaced_last_word = true;
                self.history.record(UndoRecord {
                    original_word: word.clone(),
                    replaced_word: cap,
                    trailing_delimiter: Some(delimiter),
                    restore_layout: None,
                    kind: CorrectionKind::AutoCapitalize,
                });
                return EngineAction::ReplaceText {
                    backspace_count: word.chars().count() + 1,
                    text: out_text,
                    switch_layout: None,
                    reason: "автокапитализация".to_string(),
                    elapsed_us: started.elapsed().as_micros() as u64,
                    auto_learned_word: None,
                };
            }
        }

        let _ = layout;
        EngineAction::NoOp
    }

    fn handle_undo(&mut self, started: Instant) -> EngineAction {
        let Some((rec, learned)) = self.history.pop_undo(
            self.config.layout.auto_learn_enabled,
            self.config.layout.auto_learn_threshold,
        ) else {
            return EngineAction::NoOp;
        };

        let mut learned_added = None;
        if let Some(word_to_learn) = learned {
            if self.add_user_word(&word_to_learn, WordSource::AutoLearned) {
                learned_added = Some(word_to_learn);
            }
        }

        let delim_len = usize::from(rec.trailing_delimiter.is_some());
        let erase = rec.replaced_word.chars().count() + delim_len;
        let mut restored = rec.original_word.clone();
        if let Some(d) = rec.trailing_delimiter {
            restored.push(d);
        }

        self.buffer.update_last_completed_word(&rec.original_word);
        self.just_replaced_last_word = false;

        EngineAction::ReplaceText {
            backspace_count: erase,
            text: restored,
            switch_layout: rec.restore_layout,
            reason: if learned_added.is_some() {
                "отмена (слово добавлено в белый список)".to_string()
            } else {
                "отмена последнего действия".to_string()
            },
            elapsed_us: started.elapsed().as_micros() as u64,
            auto_learned_word: learned_added,
        }
    }

    fn handle_symbol_swap(&mut self, layout: Language, started: Instant) -> EngineAction {
        if !self.config.layout.single_symbol_swap {
            return EngineAction::NoOp;
        }
        let Some(last) = self.buffer.last_char() else {
            return EngineAction::NoOp;
        };
        let Some(swapped) = swap_special_symbol(last, layout) else {
            return EngineAction::NoOp;
        };

        self.buffer.pop_char();
        self.buffer.push_word_char(swapped);

        EngineAction::ReplaceText {
            backspace_count: 1,
            text: swapped.to_string(),
            switch_layout: None,
            reason: format!("спецсимвол {last} → {swapped}"),
            elapsed_us: started.elapsed().as_micros() as u64,
            auto_learned_word: None,
        }
    }

    fn handle_manual_convert_last_word(&mut self, started: Instant) -> EngineAction {
        let (target, had_trailing_space) = if !self.buffer.current_word().is_empty() {
            (self.buffer.current_word().to_string(), false)
        } else if !self.buffer.last_completed_word().is_empty() {
            (self.buffer.last_completed_word().to_string(), true)
        } else {
            return EngineAction::NoOp;
        };

        let (converted, target_lang) = convert_layout_auto(&target);
        if converted == target {
            return EngineAction::NoOp;
        }

        let erase = target.chars().count() + usize::from(had_trailing_space);
        let mut text = converted.clone();
        if had_trailing_space {
            text.push(' ');
        }

        if had_trailing_space {
            self.buffer.update_last_completed_word(&converted);
        } else {
            self.buffer.clear_word_only();
            for c in converted.chars() {
                self.buffer.push_word_char(c);
            }
        }

        EngineAction::ReplaceText {
            backspace_count: erase,
            text,
            switch_layout: Some(target_lang),
            reason: format!("ручная смена → {}", target_lang.label()),
            elapsed_us: started.elapsed().as_micros() as u64,
            auto_learned_word: None,
        }
    }

    fn handle_manual_cycle_case(&mut self, started: Instant) -> EngineAction {
        let (target, had_trailing_space) = if !self.buffer.current_word().is_empty() {
            (self.buffer.current_word().to_string(), false)
        } else if !self.buffer.last_completed_word().is_empty() {
            (self.buffer.last_completed_word().to_string(), true)
        } else {
            return EngineAction::NoOp;
        };

        let cycled = cycle_case(&target);
        if cycled == target {
            return EngineAction::NoOp;
        }

        let erase = target.chars().count() + usize::from(had_trailing_space);
        let mut text = cycled.clone();
        if had_trailing_space {
            text.push(' ');
        }

        if had_trailing_space {
            self.buffer.update_last_completed_word(&cycled);
        } else {
            self.buffer.clear_word_only();
            for c in cycled.chars() {
                self.buffer.push_word_char(c);
            }
        }

        EngineAction::ReplaceText {
            backspace_count: erase,
            text,
            switch_layout: None,
            reason: "смена регистра".to_string(),
            elapsed_us: started.elapsed().as_micros() as u64,
            auto_learned_word: None,
        }
    }
}
