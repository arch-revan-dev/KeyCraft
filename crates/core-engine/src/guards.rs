//! Deterministic false-positive guards protecting URLs, file paths, emails,
//! code identifiers (`camelCase`, `snake_case`), numbers, and CLI flags from auto-conversion.

use crate::keymap::{is_en_letter, is_ru_letter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    EmptyOrTooLong,
    ContainsDigits,
    UrlOrDomain,
    FilePath,
    EmailAddress,
    CodeIdentifier,
    MixedScripts,
    CliFlag,
}

impl SkipReason {
    #[must_use]
    pub const fn label_ru(self) -> &'static str {
        match self {
            Self::EmptyOrTooLong => "пустое или слишком длинное",
            Self::ContainsDigits => "содержит цифры",
            Self::UrlOrDomain => "URL или домен",
            Self::FilePath => "путь к файлу",
            Self::EmailAddress => "email",
            Self::CodeIdentifier => "идентификатор кода",
            Self::MixedScripts => "смешанная раскладка",
            Self::CliFlag => "флаг командной строки",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardVerdict {
    Allow,
    Skip(SkipReason),
}

const KNOWN_TLDS: &[&str] = &[
    "com", "org", "net", "io", "dev", "app", "ru", "su", "рф", "ai", "co", "uk", "de", "fr", "jp",
    "cn", "me", "tv", "gg", "sh", "rs", "js", "ts", "py", "md", "json", "toml", "yaml", "yml",
    "exe", "dll", "png", "jpg", "jpeg", "svg", "pdf", "zip", "txt", "html", "css",
];

/// Evaluate whether `word` should be skipped by layout auto-switching / typo fixing.
#[must_use]
pub fn check_word_guards(word: &str) -> GuardVerdict {
    let char_count = word.chars().count();
    if char_count == 0 || char_count > 64 {
        return GuardVerdict::Skip(SkipReason::EmptyOrTooLong);
    }

    let lower = word.to_lowercase();

    // 1. CLI flags like `-rf`, `--verbose`
    if word.starts_with("--") || (word.starts_with('-') && char_count >= 2) {
        return GuardVerdict::Skip(SkipReason::CliFlag);
    }

    // 2. Explicit URL protocols & www.
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ftp://")
        || lower.starts_with("file://")
        || lower.starts_with("ws://")
        || lower.starts_with("wss://")
        || lower.starts_with("www.")
        || lower.contains("://")
    {
        return GuardVerdict::Skip(SkipReason::UrlOrDomain);
    }

    // 3. File paths (Windows `C:\...`, Unix `/usr/...`, `./foo`, `../bar`, `~/...`)
    if word.contains('\\')
        || word.contains('/')
        || word.starts_with("./")
        || word.starts_with("../")
        || word.starts_with("~/")
        || (word.len() >= 2 && word.as_bytes()[0].is_ascii_alphabetic() && word.as_bytes()[1] == b':')
    {
        return GuardVerdict::Skip(SkipReason::FilePath);
    }

    // 4. Email addresses (`user@domain.com`)
    if let Some(at_pos) = word.find('@') {
        if at_pos > 0 && at_pos + 1 < word.len() {
            return GuardVerdict::Skip(SkipReason::EmailAddress);
        }
    }

    // 5. Digits inside token (`v2`, `mp3`, `utf8`, `2026`, `h2o`)
    if word.chars().any(|c| c.is_ascii_digit()) {
        return GuardVerdict::Skip(SkipReason::ContainsDigits);
    }

    // 6. Code syntax tokens (`snake_case`, `std::vec`, `foo()`, `$var`, `#define`, `a=b`)
    if word.contains('_')
        || word.contains("::")
        || word.contains("->")
        || word.contains("=>")
        || word.contains('(')
        || word.contains(')')
        || word.contains('$')
        || word.contains('=')
        || word.contains('+')
        || word.contains('*')
        || word.contains('%')
        || word.contains('&')
        || word.contains('|')
        || word.contains('^')
    {
        return GuardVerdict::Skip(SkipReason::CodeIdentifier);
    }

    // 7. Domain names / file extensions (`example.com`, `main.rs`, `config.json`)
    if let Some((left, right)) = lower.rsplit_once('.') {
        if !left.is_empty()
            && !right.is_empty()
            && left.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
            && KNOWN_TLDS.contains(&right)
        {
            return GuardVerdict::Skip(SkipReason::UrlOrDomain);
        }
    }

    // 8. Mixed Cyrillic + Latin letters (e.g. `приветWorld`)
    let has_ru = word.chars().any(is_ru_letter);
    let has_en = word.chars().any(is_en_letter);
    if has_ru && has_en {
        return GuardVerdict::Skip(SkipReason::MixedScripts);
    }

    // 9. camelCase / PascalCase code identifiers (e.g. `camelCase`, `getElementById`, `HttpClient`).
    // Carefully distinguish from accidental CapsLock (`hELLO` / `пРИВЕТ`), where char[0] is lowercase
    // and ALL subsequent letters are uppercase!
    if is_camel_or_pascal_code(word) {
        return GuardVerdict::Skip(SkipReason::CodeIdentifier);
    }

    GuardVerdict::Allow
}

/// Detect `camelCase` (`myVariable`, `parseJson`) and multi-hump `PascalCase` (`HttpClient`)
/// WITHOUT triggering on accidental CapsLock (`hELLO`, `пРИВЕТ`) or normal `Title` / `UPPER`.
#[must_use]
pub fn is_camel_or_pascal_code(word: &str) -> bool {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 3 {
        return false;
    }

    // Check accidental CapsLock pattern: first char lower, ALL remaining letters upper -> NOT camelCase!
    let first_lower = letters[0].is_lowercase();
    let rest_all_upper = letters[1..].iter().all(|c| c.is_uppercase());
    if first_lower && rest_all_upper {
        return false;
    }

    // Count lower -> upper transitions inside the word (e.g. `fooBar`, `HttpClient`)
    let mut lower_to_upper = 0usize;
    let mut upper_to_lower = 0usize;
    for win in letters.windows(2) {
        if win[0].is_lowercase() && win[1].is_uppercase() {
            lower_to_upper += 1;
        } else if win[0].is_uppercase() && win[1].is_lowercase() {
            upper_to_lower += 1;
        }
    }

    // `camelCase` has at least one lower->upper followed by a lowercase (or multiple transitions).
    // `HttpClient` has upper->lower then lower->upper.
    lower_to_upper >= 1 && upper_to_lower >= 1
}
