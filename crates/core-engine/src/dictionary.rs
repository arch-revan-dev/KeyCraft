//! Embedded offline FST dictionaries (RU + EN), compact bigram/trigram
//! n-gram tables, and user whitelist / custom dictionary.

use std::collections::HashSet;

use fst::Map;

use crate::config::{Language, UserWordEntry};

static RU_FST_BYTES: &[u8] = include_bytes!("../assets/ru_freq.fst");
static EN_FST_BYTES: &[u8] = include_bytes!("../assets/en_freq.fst");
static RU_NGRAM_BYTES: &[u8] = include_bytes!("../assets/ru_ngrams.bin");
static EN_NGRAM_BYTES: &[u8] = include_bytes!("../assets/en_ngrams.bin");

const RU_ALPHABET: &[char] = &[
    'а', 'б', 'в', 'г', 'д', 'е', 'ё', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п', 'р', 'с',
    'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
];

const EN_ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

struct NgramModel {
    alphabet_len: usize,
    bigrams: Vec<i16>,
    trigrams: Vec<i16>,
}

impl NgramModel {
    fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= 6 && &bytes[0..4] == b"KCNG", "invalid ngram asset header");
        let n = bytes[5] as usize;
        let m = n + 1;
        let bi_len = m * m;
        let tri_len = m * m * m;
        let expected = 6 + (bi_len + tri_len) * 2;
        assert!(bytes.len() == expected, "corrupt ngram asset length");

        let mut offset = 6;
        let mut bigrams = Vec::with_capacity(bi_len);
        for _ in 0..bi_len {
            let val = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            bigrams.push(val);
            offset += 2;
        }
        let mut trigrams = Vec::with_capacity(tri_len);
        for _ in 0..tri_len {
            let val = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            trigrams.push(val);
            offset += 2;
        }

        Self {
            alphabet_len: n,
            bigrams,
            trigrams,
        }
    }

    fn char_index(ch: char, lang: Language) -> Option<usize> {
        let alphabet = match lang {
            Language::Ru => RU_ALPHABET,
            Language::En => EN_ALPHABET,
        };
        alphabet.iter().position(|&c| c == ch)
    }

    /// Average combined bigram+trigram log-probability per character (higher / closer to 0 is more natural).
    fn score_word(&self, word_lower: &str, lang: Language) -> f32 {
        let n = self.alphabet_len;
        let m = n + 1;
        let mut seq = Vec::with_capacity(word_lower.len() + 2);
        seq.push(n);
        let mut unknown = 0usize;
        for ch in word_lower.chars() {
            if let Some(idx) = Self::char_index(ch, lang) {
                seq.push(idx);
            } else if ch == '-' || ch == '\'' {
                continue;
            } else {
                unknown += 1;
            }
        }
        seq.push(n);

        if seq.len() <= 2 {
            return -12.0;
        }

        let mut bi_sum = 0.0f32;
        let mut bi_count = 0usize;
        for w in seq.windows(2) {
            bi_sum += f32::from(self.bigrams[w[0] * m + w[1]]) / 100.0;
            bi_count += 1;
        }

        let mut tri_sum = 0.0f32;
        let mut tri_count = 0usize;
        for w in seq.windows(3) {
            tri_sum += f32::from(self.trigrams[(w[0] * m + w[1]) * m + w[2]]) / 100.0;
            tri_count += 1;
        }

        let bi_avg = if bi_count > 0 { bi_sum / bi_count as f32 } else { -10.0 };
        let tri_avg = if tri_count > 0 { tri_sum / tri_count as f32 } else { bi_avg };

        (0.4 * bi_avg + 0.6 * tri_avg) - (unknown as f32 * 3.5)
    }
}

pub struct Dictionary {
    ru_fst: Map<&'static [u8]>,
    en_fst: Map<&'static [u8]>,
    ru_ngrams: NgramModel,
    en_ngrams: NgramModel,
    user_words: HashSet<String>,
}

impl Dictionary {
    #[must_use]
    pub fn load_embedded(user_entries: &[UserWordEntry]) -> Self {
        let ru_fst = Map::new(RU_FST_BYTES).expect("embedded ru_freq.fst must be valid");
        let en_fst = Map::new(EN_FST_BYTES).expect("embedded en_freq.fst must be valid");
        let ru_ngrams = NgramModel::from_bytes(RU_NGRAM_BYTES);
        let en_ngrams = NgramModel::from_bytes(EN_NGRAM_BYTES);

        let mut user_words = HashSet::with_capacity(user_entries.len());
        for entry in user_entries {
            let norm = entry.word.trim().to_lowercase();
            if !norm.is_empty() {
                user_words.insert(norm);
            }
        }

        Self {
            ru_fst,
            en_fst,
            ru_ngrams,
            en_ngrams,
            user_words,
        }
    }

    pub fn sync_user_words(&mut self, entries: &[UserWordEntry]) {
        self.user_words.clear();
        for entry in entries {
            let norm = entry.word.trim().to_lowercase();
            if !norm.is_empty() {
                self.user_words.insert(norm);
            }
        }
    }

    pub fn add_user_word(&mut self, word: &str) -> bool {
        let norm = word.trim().to_lowercase();
        if norm.is_empty() {
            return false;
        }
        self.user_words.insert(norm)
    }

    pub fn remove_user_word(&mut self, word: &str) -> bool {
        let norm = word.trim().to_lowercase();
        self.user_words.remove(&norm)
    }

    #[must_use]
    pub fn is_user_word(&self, word_lower: &str) -> bool {
        self.user_words.contains(word_lower)
    }

    /// Look up word frequency rank (0 = most frequent). Also treats `ё` / `е` fallback cleanly for RU.
    #[must_use]
    pub fn rank(&self, word_lower: &str, lang: Language) -> Option<u32> {
        let fst = match lang {
            Language::Ru => &self.ru_fst,
            Language::En => &self.en_fst,
        };
        if let Some(r) = fst.get(word_lower) {
            return Some(r as u32);
        }
        if lang == Language::Ru && word_lower.contains('ё') {
            let alt = word_lower.replace('ё', "е");
            if let Some(r) = fst.get(&alt) {
                return Some(r as u32);
            }
        }
        None
    }

    #[must_use]
    pub fn contains(&self, word_lower: &str, lang: Language) -> bool {
        self.is_user_word(word_lower) || self.rank(word_lower, lang).is_some()
    }

    #[must_use]
    pub fn ngram_score(&self, word_lower: &str, lang: Language) -> f32 {
        match lang {
            Language::Ru => self.ru_ngrams.score_word(word_lower, Language::Ru),
            Language::En => self.en_ngrams.score_word(word_lower, Language::En),
        }
    }

    #[must_use]
    pub fn ru_word_count(&self) -> usize {
        self.ru_fst.len()
    }

    #[must_use]
    pub fn en_word_count(&self) -> usize {
        self.en_fst.len()
    }
}
