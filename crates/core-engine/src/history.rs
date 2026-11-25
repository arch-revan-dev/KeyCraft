//! Single-step Undo history for automatic corrections and deterministic
//! auto-learning (no AI: cancelling the same word N times protects it in the user dictionary).

use std::collections::HashMap;

use crate::config::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionKind {
    LayoutSwitch,
    CapsLockFix,
    TypoFix,
    Snippet,
    AutoCapitalize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoRecord {
    pub original_word: String,
    pub replaced_word: String,
    pub trailing_delimiter: Option<char>,
    pub restore_layout: Option<Language>,
    pub kind: CorrectionKind,
}

#[derive(Default)]
pub struct CorrectionHistory {
    last_undo: Option<UndoRecord>,
    cancel_counts: HashMap<String, u8>,
}

impl CorrectionHistory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: UndoRecord) {
        self.last_undo = Some(entry);
    }

    pub fn clear_last(&mut self) {
        self.last_undo = None;
    }

    #[must_use]
    pub fn peek_last(&self) -> Option<&UndoRecord> {
        self.last_undo.as_ref()
    }

    /// Pop the last undo entry and update the deterministic auto-learn counter.
    /// Returns `(UndoRecord, Option<auto_learned_word>)`.
    pub fn pop_undo(
        &mut self,
        auto_learn_enabled: bool,
        auto_learn_threshold: u8,
    ) -> Option<(UndoRecord, Option<String>)> {
        let rec = self.last_undo.take()?;
        let mut learned = None;

        if auto_learn_enabled
            && matches!(rec.kind, CorrectionKind::LayoutSwitch | CorrectionKind::TypoFix)
        {
            let key = rec.original_word.trim().to_lowercase();
            if !key.is_empty() {
                let threshold = auto_learn_threshold.clamp(2, 10);
                let count = self.cancel_counts.entry(key.clone()).or_insert(0);
                *count = count.saturating_add(1);
                if *count >= threshold {
                    learned = Some(key);
                }
            }
        }

        Some((rec, learned))
    }

    #[must_use]
    pub fn cancel_count(&self, word: &str) -> u8 {
        self.cancel_counts
            .get(&word.trim().to_lowercase())
            .copied()
            .unwrap_or(0)
    }
}
