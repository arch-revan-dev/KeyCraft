//! Deterministic case cycling (`привет мир` → `Привет мир` → `ПРИВЕТ МИР` → `Привет Мир`),
//! sentence auto-capitalization, and smart numbered/bulleted list continuation on Enter.

/// Cycle text through 4 deterministic casing states:
/// 1. all lowercase (`привет мир`)
/// 2. Sentence case (`Привет мир`)
/// 3. ALL UPPERCASE (`ПРИВЕТ МИР`)
/// 4. Title Case (`Привет Мир`)
#[must_use]
pub fn cycle_case(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let lower = text.to_lowercase();
    let sentence = to_sentence_case(&lower);
    let upper = text.to_uppercase();
    let title = to_title_case(&lower);

    if text == lower {
        sentence
    } else if text == sentence && sentence != upper {
        upper
    } else if text == upper {
        // For single-word strings Title == Sentence, so wrap back to lower cleanly.
        if title == sentence {
            lower
        } else {
            title
        }
    } else if text == title {
        lower
    } else {
        // Arbitrary mixed case -> start cycle at Sentence case
        sentence
    }
}

#[must_use]
pub fn to_sentence_case(lower: &str) -> String {
    let mut out = String::with_capacity(lower.len());
    let mut capitalize_next = true;
    for ch in lower.chars() {
        if capitalize_next && ch.is_alphabetic() {
            for uc in ch.to_uppercase() {
                out.push(uc);
            }
            capitalize_next = false;
        } else {
            out.push(ch);
            if matches!(ch, '.' | '!' | '?' | '\n') {
                capitalize_next = true;
            }
        }
    }
    out
}

#[must_use]
pub fn to_title_case(lower: &str) -> String {
    let mut out = String::with_capacity(lower.len());
    let mut at_word_start = true;
    for ch in lower.chars() {
        if ch.is_alphabetic() {
            if at_word_start {
                for uc in ch.to_uppercase() {
                    out.push(uc);
                }
                at_word_start = false;
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
            at_word_start = !ch.is_alphanumeric() && ch != '\'';
        }
    }
    out
}

/// Action to execute when the user presses Enter inside a plain text field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartListAction {
    /// Insert the next list marker right after the newline (e.g. `2. `, `- `, `• `, `* `).
    InsertNextMarker(String),
    /// User pressed Enter on an empty list item: erase the empty marker (and stay on a clean line).
    ExitEmptyItem { erase_chars: usize },
}

/// Evaluate current line contents on `Enter` press.
#[must_use]
pub fn evaluate_smart_list_enter(current_line: &str) -> Option<SmartListAction> {
    if current_line.is_empty() {
        return None;
    }

    // Preserve leading spaces/tabs indentation
    let indent_bytes = current_line
        .bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count();
    let indent = &current_line[..indent_bytes];
    let rest = &current_line[indent_bytes..];

    // 1. Bulleted lists: `- `, `• `, `* `
    for bullet in ["- ", "• ", "* "] {
        if let Some(after_bullet) = rest.strip_prefix(bullet) {
            if after_bullet.trim().is_empty() {
                return Some(SmartListAction::ExitEmptyItem {
                    erase_chars: current_line.chars().count(),
                });
            }
            return Some(SmartListAction::InsertNextMarker(format!("{indent}{bullet}")));
        }
    }

    // 2. Numbered lists: `1. `, `23. `, `1) `
    let digit_count = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digit_count > 0 && digit_count <= 4 {
        let num_str = &rest[..digit_count];
        let after_digits = &rest[digit_count..];
        for sep in [". ", ") "] {
            if let Some(after_sep) = after_digits.strip_prefix(sep) {
                let num: u32 = num_str.parse().ok()?;
                if after_sep.trim().is_empty() {
                    return Some(SmartListAction::ExitEmptyItem {
                        erase_chars: current_line.chars().count(),
                    });
                }
                let next_num = num.saturating_add(1);
                return Some(SmartListAction::InsertNextMarker(format!(
                    "{indent}{next_num}{sep}"
                )));
            }
        }
    }

    None
}
