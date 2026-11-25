//! Deterministic offline typo correction (Damerau-Levenshtein = 1)
//! weighted by physical QWERTY / ЙЦУКЕН adjacent key distances and FST frequency ranks.

use crate::config::{Language, TypoConfig};
use crate::dictionary::Dictionary;
use crate::guards::{check_word_guards, GuardVerdict};
use crate::keymap::{adjacent_letters, is_en_letter, is_ru_letter};
use crate::switcher::apply_case_from;

const RU_LETTERS: &[char] = &[
    'а', 'б', 'в', 'г', 'д', 'е', 'ё', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п', 'р', 'с',
    'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
];

const EN_LETTERS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[derive(Debug, Clone)]
struct Candidate {
    word: String,
    rank: u32,
    score: f32,
}

/// Attempt a high-confidence deterministic DL=1 typo fix.
/// Returns `Some(corrected_word)` only when the word is unknown AND a single unambiguous
/// high-frequency dictionary correction clearly wins.
#[must_use]
pub fn fix_typo(word: &str, dict: &Dictionary, config: &TypoConfig) -> Option<String> {
    if !config.enabled {
        return None;
    }
    let min_len = config.min_word_length.max(3);
    let char_count = word.chars().count();
    if char_count < min_len || char_count > 32 {
        return None;
    }
    if check_word_guards(word) != GuardVerdict::Allow {
        return None;
    }

    // Must be purely alphabetic in a single script (no punctuation or hyphens for typo guessing).
    let all_ru = word.chars().all(is_ru_letter);
    let all_en = word.chars().all(is_en_letter);
    let lang = if all_ru {
        Language::Ru
    } else if all_en {
        Language::En
    } else {
        return None;
    };

    let lower = word.to_lowercase();
    if dict.contains(&lower, lang) {
        return None;
    }

    let chars: Vec<char> = lower.chars().collect();
    let len = chars.len();
    let mut best: Option<Candidate> = None;
    let mut second_best_score: f32 = f32::INFINITY;

    let mut consider = |cand: String, edit_cost: f32| {
        let Some(rank) = dict.rank(&cand, lang) else {
            return;
        };
        // Only auto-correct into well-established common words.
        if rank > 18_000 {
            return;
        }
        // Lower score is better: edit_cost dominates, frequency rank breaks ties smoothly.
        let freq_term = ((rank as f32) + 2.0).ln();
        let score = edit_cost * 3.0 + freq_term;

        match &mut best {
            None => {
                best = Some(Candidate {
                    word: cand,
                    rank,
                    score,
                });
            }
            Some(current) => {
                if current.word == cand {
                    if score < current.score {
                        current.score = score;
                    }
                } else if score < current.score {
                    second_best_score = second_best_score.min(current.score);
                    *current = Candidate {
                        word: cand,
                        rank,
                        score,
                    };
                } else if score < second_best_score {
                    second_best_score = score;
                }
            }
        }
    };

    // 1. Adjacent transpositions (`teh` -> `the`, `привте` -> `привет`)
    for i in 0..(len - 1) {
        if chars[i] == chars[i + 1] {
            continue;
        }
        let mut buf = chars.clone();
        buf.swap(i, i + 1);
        consider(buf.into_iter().collect(), 0.78);
    }

    // 2. Deletions — extra accidental letter (`приввет` -> `привет`, `helllo` -> `hello`)
    for i in 0..len {
        let is_duplicate = (i > 0 && chars[i] == chars[i - 1])
            || (i + 1 < len && chars[i] == chars[i + 1]);
        let mut s = String::with_capacity(lower.len());
        for (j, &c) in chars.iter().enumerate() {
            if j != i {
                s.push(c);
            }
        }
        consider(s, if is_duplicate { 0.80 } else { 1.05 });
    }

    // 3. Adjacent-key substitutions (`привнт` -> `привет`, `wprld` -> `world`)
    //    plus common Russian unstressed vowel confusions (`а↔о`, `и↔е`, `е↔ё`: `сматришь` -> `смотришь`, `чта` -> `что`).
    for i in 0..len {
        let neighbors = adjacent_letters(chars[i], lang);
        for (neighbor, dist) in neighbors {
            let mut buf = chars.clone();
            buf[i] = neighbor;
            let cost = 0.82 + 0.22 * dist;
            consider(buf.into_iter().collect(), cost);
        }

        if lang == Language::Ru {
            let vowel_alts: &[char] = match chars[i] {
                'а' => &['о'],
                'о' => &['а'],
                'и' => &['е'],
                'е' => &['и', 'ё'],
                'ё' => &['е'],
                _ => &[],
            };
            for &alt in vowel_alts {
                let mut buf = chars.clone();
                buf[i] = alt;
                consider(buf.into_iter().collect(), 1.15);
            }
        }
    }

    // 4. Insertions — omitted letter (`helo` -> `hello`, `привт` -> `привет`)
    let alphabet = match lang {
        Language::Ru => RU_LETTERS,
        Language::En => EN_LETTERS,
    };
    let mut cand_buf = String::with_capacity(lower.len() + 4);
    for pos in 0..=len {
        for &ins in alphabet {
            cand_buf.clear();
            for &c in &chars[..pos] {
                cand_buf.push(c);
            }
            cand_buf.push(ins);
            for &c in &chars[pos..] {
                cand_buf.push(c);
            }
            let is_doubled = (pos > 0 && chars[pos - 1] == ins) || (pos < len && chars[pos] == ins);
            let cost = if is_doubled { 0.85 } else { 1.12 };
            if dict.rank(&cand_buf, lang).is_some() {
                consider(cand_buf.clone(), cost);
            }
        }
    }

    let winner = best?;

    // Strict confidence check:
    // - For 3-letter words, require a very common winner (rank <= 2500) and a large margin to avoid corrupting abbreviations.
    // - For 4+ letter words, require clear margin over second candidate if one exists.
    if char_count == 3 {
        if winner.rank > 2_500 {
            return None;
        }
        if second_best_score.is_finite() && (second_best_score - winner.score) < 1.8 {
            return None;
        }
    } else if second_best_score.is_finite() && (second_best_score - winner.score) < 1.15 {
        return None;
    }

    let (cased, _) = apply_case_from(word, &winner.word, false);
    Some(cased)
}
