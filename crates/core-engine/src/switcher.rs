//! Deterministic RU ↔ EN auto-layout switcher, casing preservation,
//! accidental CapsLock repair (`пРИВЕТ` -> `Привет`), and trailing punctuation handling.

use crate::config::{Language, LayoutConfig};
use crate::dictionary::Dictionary;
use crate::guards::{check_word_guards, GuardVerdict};
use crate::keymap::{
    convert_en_to_ru, convert_ru_to_en, is_en_letter, is_en_punct_mapping_to_ru_letter,
    is_ru_letter, script_letter_counts,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchKind {
    LayoutSwitch { target_layout: Language },
    CapsLockFix,
    LayoutAndCapsLockFix { target_layout: Language },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchDecision {
    pub replacement: String,
    pub kind: SwitchKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CasePattern {
    Lower,
    Capitalized,
    AllUpper,
    AccidentalCapsLock,
    StickyLeadingShift,
    Mixed,
}

fn detect_case_pattern(word: &str) -> CasePattern {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return CasePattern::Lower;
    }
    if letters.iter().all(|c| c.is_lowercase()) {
        return CasePattern::Lower;
    }
    if letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase()) {
        return CasePattern::AllUpper;
    }
    if letters[0].is_uppercase() && letters[1..].iter().all(|c| c.is_lowercase()) {
        return CasePattern::Capitalized;
    }
    if letters.len() >= 3 && letters[0].is_lowercase() && letters[1..].iter().all(|c| c.is_uppercase()) {
        return CasePattern::AccidentalCapsLock;
    }
    // Sticky Shift at word start: 2..=3 leading uppercase letters followed by 1+ ALL-lowercase letters
    // (e.g. `ЧТОбы`, `ПРивет`, `ЧТо`, `HEllo`). Never matches `HttpClient` because rest must be all-lower.
    let leading_upper = letters.iter().take_while(|c| c.is_uppercase()).count();
    if (2..=3).contains(&leading_upper)
        && letters.len() > leading_upper
        && letters[leading_upper..].iter().all(|c| c.is_lowercase())
    {
        return CasePattern::StickyLeadingShift;
    }
    CasePattern::Mixed
}

#[must_use]
pub fn apply_case_from(source: &str, converted: &str, fix_capslock: bool) -> (String, bool) {
    match detect_case_pattern(source) {
        CasePattern::Lower => (converted.to_lowercase(), false),
        CasePattern::AllUpper => (converted.to_uppercase(), false),
        CasePattern::Capitalized => (capitalize_first(converted), false),
        CasePattern::AccidentalCapsLock | CasePattern::StickyLeadingShift if fix_capslock => {
            (capitalize_first(converted), true)
        }
        CasePattern::AccidentalCapsLock | CasePattern::StickyLeadingShift | CasePattern::Mixed => {
            (converted.to_string(), false)
        }
    }
}

#[must_use]
pub fn capitalize_first(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::with_capacity(lower.len());
    for uc in first.to_uppercase() {
        out.push(uc);
    }
    out.push_str(chars.as_str());
    out
}

fn has_vowel(word_lower: &str, lang: Language) -> bool {
    match lang {
        Language::Ru => word_lower
            .chars()
            .any(|c| matches!(c, 'а' | 'е' | 'ё' | 'и' | 'о' | 'у' | 'ы' | 'э' | 'ю' | 'я')),
        Language::En => word_lower
            .chars()
            .any(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')),
    }
}

/// Split trailing sentence punctuation (`,` `.` `!` `?` `;` `:` `)`) from a word when not needed as RU letters.
fn split_trailing_punct(word: &str) -> (&str, &str) {
    let mut split_at = word.len();
    for (idx, ch) in word.char_indices().rev() {
        if matches!(ch, ',' | '.' | '!' | '?' | ';' | ':' | ')' | '»' | '"') {
            split_at = idx;
        } else {
            break;
        }
    }
    (&word[..split_at], &word[split_at..])
}

/// Analyze a completed word and decide whether to auto-switch layout and/or fix accidental CapsLock.
#[must_use]
pub fn evaluate_layout_switch(
    word: &str,
    dict: &Dictionary,
    config: &LayoutConfig,
) -> Option<SwitchDecision> {
    if !config.auto_switch && !config.fix_capslock {
        return None;
    }

    // 1. Check accidental CapsLock (`пРИВЕТ` -> `Привет`, `hELLO` -> `Hello`) or sticky Shift
    //    (`ЧТОбы` -> `Чтобы`, `ПРивет` -> `Привет`, `HEllo` -> `Hello`) on an ALREADY valid dictionary word.
    let (core_stripped, trailing_punct) = split_trailing_punct(word);
    if config.fix_capslock && !core_stripped.is_empty() {
        let pat = detect_case_pattern(core_stripped);
        if matches!(pat, CasePattern::AccidentalCapsLock | CasePattern::StickyLeadingShift) {
            let core_lower = core_stripped.to_lowercase();
            let (ru_c, en_c) = script_letter_counts(core_stripped);
            let native_lang = if ru_c > en_c { Language::Ru } else { Language::En };
            // For sticky leading Shift, require an exact embedded dictionary match (never mutate user terms/acronyms).
            let valid_target = match pat {
                CasePattern::AccidentalCapsLock => dict.contains(&core_lower, native_lang),
                CasePattern::StickyLeadingShift => {
                    !dict.is_user_word(&core_lower) && dict.rank(&core_lower, native_lang).is_some()
                }
                _ => false,
            };
            if valid_target {
                let mut fixed = capitalize_first(core_stripped);
                fixed.push_str(trailing_punct);
                return Some(SwitchDecision {
                    replacement: fixed,
                    kind: SwitchKind::CapsLockFix,
                });
            }
        }
    }

    if !config.auto_switch {
        return None;
    }

    // 2. If there is trailing punctuation (like `ghbdtn,` or `k.,k.`), prefer exact dictionary hits first:
    //    - Full token exact dictionary hit (`k.,k.` -> `люблю`) wins first.
    //    - Stripped stem exact dictionary hit (`ghbdtn` + `,` -> `привет,`) wins next.
    //    - Only if neither hits the dictionary, allow n-gram fallback on stripped stem (or full token).
    if !trailing_punct.is_empty() && !core_stripped.is_empty() {
        if let Some(decision) = try_switch_token(word, "", dict, config, false) {
            return Some(decision);
        }
        if let Some(decision) = try_switch_token(core_stripped, trailing_punct, dict, config, true) {
            return Some(decision);
        }
        return try_switch_token(word, "", dict, config, true);
    }

    try_switch_token(word, "", dict, config, true)
}

fn try_switch_token(
    token: &str,
    keep_suffix: &str,
    dict: &Dictionary,
    config: &LayoutConfig,
    allow_ngram_fallback: bool,
) -> Option<SwitchDecision> {
    if check_word_guards(token) != GuardVerdict::Allow {
        return None;
    }

    let token_lower = token.to_lowercase();

    // User whitelist always protects exact word from switching.
    if dict.is_user_word(&token_lower) {
        return None;
    }

    let (ru_letters, en_letters) = script_letter_counts(token);
    let en_ru_punct_count = token
        .chars()
        .filter(|&c| is_en_punct_mapping_to_ru_letter(c))
        .count();

    // Determine source & target language.
    let (source_lang, target_lang, raw_converted) = if ru_letters > 0 && en_letters == 0 {
        (Language::Ru, Language::En, convert_ru_to_en(token))
    } else if (en_letters > 0 || en_ru_punct_count >= 2) && ru_letters == 0 {
        // Must contain only EN letters or EN keys that map to RU letters (or apostrophe/hyphen)
        if !token.chars().all(|c| {
            is_en_letter(c) || is_en_punct_mapping_to_ru_letter(c) || c == '-'
        }) {
            return None;
        }
        (Language::En, Language::Ru, convert_en_to_ru(token))
    } else {
        return None;
    };

    // Never auto-switch single-letter ambiguous tokens (e.g. `a`, `i`, `в`, `я`, `и`, `к`, `с`, `у`, `о`)
    // unless it's a single non-letter symbol? No, single-letter auto-switch causes massive false positives;
    // require >= 2 chars for automatic layout switch!
    if token.chars().count() < 2 {
        return None;
    }

    // Verify converted token consists of valid target-language word characters.
    let conv_lower = raw_converted.to_lowercase();
    let valid_target_chars = match target_lang {
        Language::Ru => conv_lower.chars().all(|c| is_ru_letter(c) || c == '-'),
        Language::En => conv_lower
            .chars()
            .all(|c| is_en_letter(c) || c == '\'' || c == '-'),
    };
    if !valid_target_chars {
        return None;
    }

    let src_rank = dict.rank(&token_lower, source_lang);
    let dst_rank = dict.rank(&conv_lower, target_lang);
    let dst_is_user = dict.is_user_word(&conv_lower);

    // If the original token is ALREADY in its own language dictionary, do not switch —
    // EXCEPT when it is rare/archaic/OCR noise (rank >= 2000) and the converted word is an
    // ultra-common core word (rank <= 250, e.g. `ye` -> `ну`, `ds` -> `вы`, `vs` -> `мы`)
    // dominating by at least 25x in rank.
    if let Some(r_src) = src_rank {
        let dominated_by_core = dst_rank.is_some_and(|r_dst| {
            r_dst <= 250 && r_src >= 2_000 && r_src >= r_dst.saturating_mul(25)
        });
        if !dominated_by_core {
            return None;
        }
    }

    // Decision Path A: Converted word is in user dictionary or embedded FST dictionary -> high-confidence switch!
    // For 2-letter words, require the target to be a top-400 core word (or explicit user word) so Russian
    // abbreviations like `пк`, `тк`, `вк`, `тг`, `лс` never falsely switch into obscure EN abbreviations (`gr`, `nr`, `dr`).
    let is_two_letter = conv_lower.chars().count() == 2;
    let in_target_dict = if is_two_letter {
        dst_is_user || dst_rank.is_some_and(|r| r <= 400)
    } else {
        dst_is_user || dst_rank.is_some()
    };

    // Decision Path B: Neither word is in FST (e.g. long Russian inflection or rare surname, len >= 4):
    // use deterministic bigram+trigram log-probability comparison.
    let ngram_approved = if allow_ngram_fallback
        && !in_target_dict
        && conv_lower.chars().count() >= 4
        && has_vowel(&conv_lower, target_lang)
    {
        // For EN->RU with punctuation characters (`[` `;` `'` `,` `.` `` ` ``) that cannot appear inside an English word,
        // the source EN n-gram score automatically gets penalized, making valid Russian n-grams win cleanly.
        let src_score = dict.ngram_score(&token_lower, source_lang);
        let dst_score = dict.ngram_score(&conv_lower, target_lang);
        dst_score >= -4.2 && (dst_score - src_score) >= 1.35
    } else {
        false
    };

    if !in_target_dict && !ngram_approved {
        return None;
    }

    // Reconstruct final string with exact case preservation (or accidental CapsLock repair).
    // Note: when converting EN punctuation keys like `[` `{` `'` `"` `,` `<` `.` `>` to RU `х/Х/э/Э/б/Б/ю/Ю`,
    // `raw_converted` already preserves per-key shift state, while `apply_case_from` handles clean
    // `Ghbdtn` -> `Привет`, `GHBDTN` -> `ПРИВЕТ`, `gHBDTN` -> `Привет`.
    let (mut final_word, fixed_caps) = if token.chars().all(|c| c.is_alphabetic()) {
        apply_case_from(token, &raw_converted, config.fix_capslock)
    } else {
        // Token includes shift-paired punctuation keys (e.g. `k.,k.`, `дуеэы`, `'nj`):
        // check accidental CapsLock on the converted result itself.
        let pat = detect_case_pattern(&raw_converted);
        if config.fix_capslock && pat == CasePattern::AccidentalCapsLock {
            (capitalize_first(&raw_converted), true)
        } else {
            (raw_converted, false)
        }
    };

    final_word.push_str(keep_suffix);

    let kind = if fixed_caps {
        SwitchKind::LayoutAndCapsLockFix {
            target_layout: target_lang,
        }
    } else {
        SwitchKind::LayoutSwitch {
            target_layout: target_lang,
        }
    };

    Some(SwitchDecision {
        replacement: final_word,
        kind,
    })
}
