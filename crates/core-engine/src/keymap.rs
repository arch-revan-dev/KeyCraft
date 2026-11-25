//! Bidirectional physical keymap between US QWERTY and Russian ЙЦУКЕН,
//! standalone special-symbol pairing (`№` ↔ `#`, `"` ↔ `@`, `;` ↔ `$`),
//! and 2D physical key coordinates for keyboard-aware typo distance.

use crate::config::Language;

/// Convert a single character from US QWERTY to Russian ЙЦУКЕН (preserving shift state).
#[must_use]
pub const fn en_char_to_ru(ch: char) -> char {
    match ch {
        '`' => 'ё',
        '~' => 'Ё',
        'q' => 'й',
        'Q' => 'Й',
        'w' => 'ц',
        'W' => 'Ц',
        'e' => 'у',
        'E' => 'У',
        'r' => 'к',
        'R' => 'К',
        't' => 'е',
        'T' => 'Е',
        'y' => 'н',
        'Y' => 'Н',
        'u' => 'г',
        'U' => 'Г',
        'i' => 'ш',
        'I' => 'Ш',
        'o' => 'щ',
        'O' => 'Щ',
        'p' => 'з',
        'P' => 'З',
        '[' => 'х',
        '{' => 'Х',
        ']' => 'ъ',
        '}' => 'Ъ',
        'a' => 'ф',
        'A' => 'Ф',
        's' => 'ы',
        'S' => 'Ы',
        'd' => 'в',
        'D' => 'В',
        'f' => 'а',
        'F' => 'А',
        'g' => 'п',
        'G' => 'П',
        'h' => 'р',
        'H' => 'Р',
        'j' => 'о',
        'J' => 'О',
        'k' => 'л',
        'K' => 'Л',
        'l' => 'д',
        'L' => 'Д',
        ';' => 'ж',
        ':' => 'Ж',
        '\'' => 'э',
        '"' => 'Э',
        'z' => 'я',
        'Z' => 'Я',
        'x' => 'ч',
        'X' => 'Ч',
        'c' => 'с',
        'C' => 'С',
        'v' => 'м',
        'V' => 'М',
        'b' => 'и',
        'B' => 'И',
        'n' => 'т',
        'N' => 'Т',
        'm' => 'ь',
        'M' => 'Ь',
        ',' => 'б',
        '<' => 'Б',
        '.' => 'ю',
        '>' => 'Ю',
        '/' => '.',
        '?' => ',',
        '@' => '"',
        '#' => '№',
        '$' => ';',
        '^' => ':',
        '&' => '?',
        other => other,
    }
}

/// Convert a single character from Russian ЙЦУКЕН to US QWERTY (preserving shift state).
#[must_use]
pub const fn ru_char_to_en(ch: char) -> char {
    match ch {
        'ё' => '`',
        'Ё' => '~',
        'й' => 'q',
        'Й' => 'Q',
        'ц' => 'w',
        'Ц' => 'W',
        'у' => 'e',
        'У' => 'E',
        'к' => 'r',
        'К' => 'R',
        'е' => 't',
        'Е' => 'T',
        'н' => 'y',
        'Н' => 'Y',
        'г' => 'u',
        'Г' => 'U',
        'ш' => 'i',
        'Ш' => 'I',
        'щ' => 'o',
        'Щ' => 'O',
        'з' => 'p',
        'З' => 'P',
        'х' => '[',
        'Х' => '{',
        'ъ' => ']',
        'Ъ' => '}',
        'ф' => 'a',
        'Ф' => 'A',
        'ы' => 's',
        'Ы' => 'S',
        'в' => 'd',
        'В' => 'D',
        'а' => 'f',
        'А' => 'F',
        'п' => 'g',
        'П' => 'G',
        'р' => 'h',
        'Р' => 'H',
        'о' => 'j',
        'О' => 'J',
        'л' => 'k',
        'Л' => 'K',
        'д' => 'l',
        'Д' => 'L',
        'ж' => ';',
        'Ж' => ':',
        'э' => '\'',
        'Э' => '"',
        'я' => 'z',
        'Я' => 'Z',
        'ч' => 'x',
        'Ч' => 'X',
        'с' => 'c',
        'С' => 'C',
        'м' => 'v',
        'М' => 'V',
        'и' => 'b',
        'И' => 'B',
        'т' => 'n',
        'Т' => 'N',
        'ь' => 'm',
        'Ь' => 'M',
        'б' => ',',
        'Б' => '<',
        'ю' => '.',
        'Ю' => '>',
        '№' => '#',
        '"' => '@',
        ';' => '$',
        ':' => '^',
        '?' => '&',
        '.' => '/',
        ',' => '?',
        other => other,
    }
}

/// Convert an entire string from EN to RU layout.
#[must_use]
pub fn convert_en_to_ru(s: &str) -> String {
    s.chars().map(en_char_to_ru).collect()
}

/// Convert an entire string from RU to EN layout.
#[must_use]
pub fn convert_ru_to_en(s: &str) -> String {
    s.chars().map(ru_char_to_en).collect()
}

/// Convert a string into the opposite layout based on detected dominant script.
#[must_use]
pub fn convert_layout_auto(s: &str) -> (String, Language) {
    let (ru_count, en_count) = script_letter_counts(s);
    if ru_count > en_count {
        (convert_ru_to_en(s), Language::En)
    } else {
        (convert_en_to_ru(s), Language::Ru)
    }
}

/// Post-factum standalone special-symbol swap (e.g. `№` -> `#`, `"` -> `@`, `;` -> `$`).
/// Uses the current active keyboard layout to disambiguate symbols shared between layouts.
#[must_use]
pub const fn swap_special_symbol(ch: char, current_layout: Language) -> Option<char> {
    match current_layout {
        Language::Ru => match ch {
            '№' => Some('#'),
            '"' => Some('@'),
            ';' => Some('$'),
            ':' => Some('^'),
            '?' => Some('&'),
            '.' => Some('/'),
            ',' => Some('?'),
            'ё' => Some('`'),
            'Ё' => Some('~'),
            'х' => Some('['),
            'Х' => Some('{'),
            'ъ' => Some(']'),
            'Ъ' => Some('}'),
            'ж' => Some(';'),
            'Ж' => Some(':'),
            'э' => Some('\''),
            'Э' => Some('"'),
            'б' => Some(','),
            'Б' => Some('<'),
            'ю' => Some('.'),
            'Ю' => Some('>'),
            _ => None,
        },
        Language::En => match ch {
            '#' => Some('№'),
            '@' => Some('"'),
            '$' => Some(';'),
            '^' => Some(':'),
            '&' => Some('?'),
            '/' => Some('.'),
            '?' => Some(','),
            '`' => Some('ё'),
            '~' => Some('Ё'),
            '[' => Some('х'),
            '{' => Some('Х'),
            ']' => Some('ъ'),
            '}' => Some('Ъ'),
            ';' => Some('ж'),
            ':' => Some('Ж'),
            '\'' => Some('э'),
            '"' => Some('Э'),
            ',' => Some('б'),
            '<' => Some('Б'),
            '.' => Some('ю'),
            '>' => Some('Ю'),
            _ => None,
        },
    }
}

#[must_use]
pub const fn is_ru_letter(ch: char) -> bool {
    matches!(ch, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё')
}

#[must_use]
pub const fn is_en_letter(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

/// Characters on US QWERTY that physically produce Cyrillic letters on ЙЦУКЕН.
#[must_use]
pub const fn is_en_punct_mapping_to_ru_letter(ch: char) -> bool {
    matches!(
        ch,
        '`' | '~' | '[' | '{' | ']' | '}' | ';' | ':' | '\'' | '"' | ',' | '<' | '.' | '>'
    )
}

#[must_use]
pub fn script_letter_counts(s: &str) -> (usize, usize) {
    let mut ru = 0usize;
    let mut en = 0usize;
    for ch in s.chars() {
        if is_ru_letter(ch) {
            ru += 1;
        } else if is_en_letter(ch) {
            en += 1;
        }
    }
    (ru, en)
}

/// 2D physical coordinates (row, col with staggered QWERTY offsets) for lowercase letters.
#[must_use]
pub fn key_coords(ch: char, lang: Language) -> Option<(f32, f32)> {
    let key = match lang {
        Language::En => ch,
        Language::Ru => ru_char_to_en(ch),
    };
    let pos = match key {
        '`' => (0.0, 0.0),
        'q' => (1.0, 0.0),
        'w' => (1.0, 1.0),
        'e' => (1.0, 2.0),
        'r' => (1.0, 3.0),
        't' => (1.0, 4.0),
        'y' => (1.0, 5.0),
        'u' => (1.0, 6.0),
        'i' => (1.0, 7.0),
        'o' => (1.0, 8.0),
        'p' => (1.0, 9.0),
        '[' => (1.0, 10.0),
        ']' => (1.0, 11.0),
        'a' => (2.0, 0.25),
        's' => (2.0, 1.25),
        'd' => (2.0, 2.25),
        'f' => (2.0, 3.25),
        'g' => (2.0, 4.25),
        'h' => (2.0, 5.25),
        'j' => (2.0, 6.25),
        'k' => (2.0, 7.25),
        'l' => (2.0, 8.25),
        ';' => (2.0, 9.25),
        '\'' => (2.0, 10.25),
        'z' => (3.0, 0.75),
        'x' => (3.0, 1.75),
        'c' => (3.0, 2.75),
        'v' => (3.0, 3.75),
        'b' => (3.0, 4.75),
        'n' => (3.0, 5.75),
        'm' => (3.0, 6.75),
        ',' => (3.0, 7.75),
        '.' => (3.0, 8.75),
        _ => return None,
    };
    Some(pos)
}

/// Euclidean distance between two keys on the physical keyboard.
#[must_use]
pub fn key_distance(a: char, b: char, lang: Language) -> Option<f32> {
    let (r1, c1) = key_coords(a, lang)?;
    let (r2, c2) = key_coords(b, lang)?;
    let dr = r1 - r2;
    let dc = c1 - c2;
    Some((dr * dr + dc * dc).sqrt())
}

/// Return all lowercase letters of `lang` physically adjacent to `ch` (distance <= 1.55).
#[must_use]
pub fn adjacent_letters(ch: char, lang: Language) -> Vec<(char, f32)> {
    const RU_CHARS: &[char] = &[
        'а', 'б', 'в', 'г', 'д', 'е', 'ё', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п', 'р',
        'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
    ];
    const EN_CHARS: &[char] = &[
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    let alphabet = match lang {
        Language::Ru => RU_CHARS,
        Language::En => EN_CHARS,
    };

    let mut out = Vec::with_capacity(8);
    for &cand in alphabet {
        if cand == ch {
            continue;
        }
        if let Some(d) = key_distance(ch, cand, lang) {
            if d <= 1.55 {
                out.push((cand, d));
            }
        }
    }
    out
}
