//! Deterministic text expander / snippet matcher supporting:
//! - Multiple triggers per snippet (`р- Р-` -> `₽`)
//! - Activation modes: `Space`, `Tab`, `Instant`
//! - Cross-layout matching (`+админ` also fires on `+flvby` when `any_layout = true`)

use crate::config::{SnippetActivation, SnippetEntry};
use crate::keymap::{convert_en_to_ru, convert_ru_to_en, script_letter_counts};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetMatch {
    pub trigger_char_len: usize,
    pub replacement: String,
    pub snippet_id: String,
}

#[derive(Debug, Clone)]
struct CompiledTrigger {
    trigger: String,
    trigger_chars: usize,
    replacement: String,
    activation: SnippetActivation,
    snippet_id: String,
}

pub struct SnippetMatcher {
    entries: Vec<CompiledTrigger>,
}

impl SnippetMatcher {
    #[must_use]
    pub fn compile(snippets: &[SnippetEntry]) -> Self {
        let mut entries = Vec::new();
        for snip in snippets {
            if !snip.enabled {
                continue;
            }
            for raw_trigger in &snip.triggers {
                let trig = raw_trigger.trim();
                if trig.is_empty() {
                    continue;
                }
                entries.push(CompiledTrigger {
                    trigger: trig.to_string(),
                    trigger_chars: trig.chars().count(),
                    replacement: snip.replacement.clone(),
                    activation: snip.activation,
                    snippet_id: snip.id.clone(),
                });

                if snip.any_layout {
                    let (ru_c, en_c) = script_letter_counts(trig);
                    let mapped = if ru_c >= en_c {
                        convert_ru_to_en(trig)
                    } else {
                        convert_en_to_ru(trig)
                    };
                    if mapped != trig && !mapped.is_empty() {
                        entries.push(CompiledTrigger {
                            trigger: mapped.clone(),
                            trigger_chars: mapped.chars().count(),
                            replacement: snip.replacement.clone(),
                            activation: snip.activation,
                            snippet_id: snip.id.clone(),
                        });
                    }
                }
            }
        }
        // Longest triggers take precedence so `---` wins over `--`.
        entries.sort_by_key(|a| std::cmp::Reverse(a.trigger_chars));
        Self { entries }
    }

    /// Check instant triggers matching the suffix of the current word buffer.
    #[must_use]
    pub fn match_instant(&self, buffer_word: &str) -> Option<SnippetMatch> {
        for item in &self.entries {
            if item.activation != SnippetActivation::Instant {
                continue;
            }
            if buffer_word.ends_with(&item.trigger) {
                return Some(SnippetMatch {
                    trigger_char_len: item.trigger_chars,
                    replacement: item.replacement.clone(),
                    snippet_id: item.snippet_id.clone(),
                });
            }
        }
        None
    }

    /// Check Space- or Tab-activated snippets matching the exact completed word.
    #[must_use]
    pub fn match_on_delimiter(
        &self,
        word: &str,
        activation: SnippetActivation,
    ) -> Option<SnippetMatch> {
        for item in &self.entries {
            if item.activation != activation {
                continue;
            }
            if word == item.trigger {
                return Some(SnippetMatch {
                    trigger_char_len: item.trigger_chars,
                    replacement: item.replacement.clone(),
                    snippet_id: item.snippet_id.clone(),
                });
            }
        }
        None
    }
}
