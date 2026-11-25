//! Zero-Keylogger RAM-only word and line buffer state machine.
//!
//! Security invariants:
//! - Maximum 64 characters in word buffer (and 128 in ephemeral line prefix for smart lists).
//! - All memory is explicitly overwritten with `zeroize` on reset, focus loss, click, or password field entry.
//! - Buffer contents are NEVER serialized or logged.

use zeroize::Zeroize;

pub const MAX_WORD_CHARS: usize = 64;
pub const MAX_LINE_CHARS: usize = 128;

#[derive(Default)]
pub struct InputBuffer {
    word: String,
    line: String,
    last_char: Option<char>,
    last_completed_word: String,
    sentence_start: bool,
    overflowed: bool,
}

impl Drop for InputBuffer {
    fn drop(&mut self) {
        self.clear_all();
    }
}

impl InputBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            word: String::with_capacity(MAX_WORD_CHARS),
            line: String::with_capacity(MAX_LINE_CHARS),
            last_char: None,
            last_completed_word: String::with_capacity(MAX_WORD_CHARS),
            sentence_start: false,
            overflowed: false,
        }
    }

    pub fn push_word_char(&mut self, ch: char) {
        self.last_char = Some(ch);
        self.push_line_char(ch);
        if self.overflowed {
            return;
        }
        if self.word.chars().count() >= MAX_WORD_CHARS {
            self.word.zeroize();
            self.overflowed = true;
            return;
        }
        self.word.push(ch);
    }

    pub fn push_line_char(&mut self, ch: char) {
        if self.line.chars().count() >= MAX_LINE_CHARS {
            self.line.zeroize();
        }
        self.line.push(ch);
    }

    pub fn pop_char(&mut self) {
        self.word.pop();
        self.line.pop();
        self.last_char = self.word.chars().last().or_else(|| self.line.chars().last());
        if self.word.is_empty() {
            self.overflowed = false;
        }
    }

    #[must_use]
    pub fn current_word(&self) -> &str {
        if self.overflowed {
            ""
        } else {
            &self.word
        }
    }

    #[must_use]
    pub fn current_line(&self) -> &str {
        &self.line
    }

    #[must_use]
    pub fn last_char(&self) -> Option<char> {
        self.last_char
    }

    pub fn set_last_char(&mut self, ch: Option<char>) {
        self.last_char = ch;
    }

    #[must_use]
    pub fn last_completed_word(&self) -> &str {
        &self.last_completed_word
    }

    #[must_use]
    pub fn is_sentence_start(&self) -> bool {
        self.sentence_start
    }

    pub fn set_sentence_start(&mut self, value: bool) {
        self.sentence_start = value;
    }

    /// Take the current word for processing, recording it as the last completed word
    /// (in RAM only) and zeroizing the active word buffer.
    pub fn commit_word(&mut self) -> String {
        let taken = if self.overflowed {
            String::new()
        } else {
            self.word.clone()
        };
        self.word.zeroize();
        self.overflowed = false;
        self.last_completed_word.zeroize();
        if !taken.is_empty() {
            self.last_completed_word.push_str(&taken);
        }
        taken
    }

    pub fn update_last_completed_word(&mut self, word: &str) {
        self.last_completed_word.zeroize();
        self.last_completed_word.push_str(word);
    }

    pub fn clear_word_only(&mut self) {
        self.word.zeroize();
        self.overflowed = false;
    }

    pub fn clear_line(&mut self) {
        self.line.zeroize();
    }

    /// Full zeroization on mouse click, focus change, navigation key, pause, or password field.
    pub fn clear_all(&mut self) {
        self.word.zeroize();
        self.line.zeroize();
        self.last_completed_word.zeroize();
        self.last_char = None;
        self.sentence_start = false;
        self.overflowed = false;
    }
}
