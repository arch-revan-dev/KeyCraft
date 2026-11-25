//! Offline dictionary & n-gram binary index builder for KeyCraft.
//!
//! Reads raw UTF-8 frequency lists from `data/wordlists/` and emits compact
//! deterministic binary artifacts into `crates/core-engine/assets/`:
//! - `ru_freq.fst` / `en_freq.fst`: FST mapping lowercase word -> frequency rank (u64, 0 = most frequent)
//! - `ru_ngrams.bin` / `en_ngrams.bin`: compact bigram + trigram log-probability tables
//!
//! Zero runtime network dependency; 100% deterministic.

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use fst::MapBuilder;

const RU_ALPHABET: &[char] = &[
    'а', 'б', 'в', 'г', 'д', 'е', 'ё', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п', 'р', 'с',
    'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
];

const EN_ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

/// Curated everyday Russian words (including `ё` forms and high-frequency words)
/// guaranteed to be present in `ru_freq.fst` with strong frequency priority.
const RU_CURATED: &[(&str, u64)] = &[
    ("привет", 80_000),
    ("пока", 70_000),
    ("спасибо", 90_000),
    ("пожалуйста", 75_000),
    ("хорошо", 85_000),
    ("нормально", 40_000),
    ("отлично", 45_000),
    ("конечно", 60_000),
    ("сегодня", 65_000),
    ("завтра", 55_000),
    ("вчера", 50_000),
    ("сейчас", 80_000),
    ("потом", 55_000),
    ("здесь", 65_000),
    ("там", 70_000),
    ("ещё", 95_000),
    ("всё", 120_000),
    ("её", 90_000),
    ("нём", 30_000),
    ("вдвоём", 10_000),
    ("приём", 18_000),
    ("самолёт", 15_000),
    ("вертолёт", 12_000),
    ("чёрный", 25_000),
    ("жёлтый", 15_000),
    ("тёплый", 18_000),
    ("лёгкий", 20_000),
    ("мёд", 12_000),
    ("лёд", 15_000),
    ("счёт", 25_000),
    ("отчёт", 22_000),
    ("учёт", 18_000),
    ("вперёд", 35_000),
    ("полёт", 15_000),
    ("живёт", 25_000),
    ("идёт", 40_000),
    ("даёт", 25_000),
    ("берёт", 20_000),
    ("ведёт", 20_000),
    ("ждёт", 25_000),
    ("найдёт", 20_000),
    ("поймёт", 20_000),
    ("придёт", 30_000),
    ("пройдёт", 25_000),
    ("вернётся", 20_000),
    ("остаётся", 20_000),
    ("получится", 35_000),
    ("программа", 30_000),
    ("приложение", 28_000),
    ("клавиатура", 25_000),
    ("раскладка", 25_000),
    ("скриншот", 22_000),
    ("настройки", 30_000),
    ("словарь", 25_000),
    ("опечатка", 20_000),
    ("сниппет", 15_000),
    ("буфер", 20_000),
    ("файл", 35_000),
    ("папка", 25_000),
    ("система", 35_000),
    ("проект", 35_000),
    ("задача", 35_000),
    ("работа", 55_000),
    ("время", 75_000),
    ("человек", 70_000),
    ("мир", 50_000),
    ("день", 65_000),
    ("дом", 60_000),
    ("год", 70_000),
    ("раз", 65_000),
    ("друг", 55_000),
    ("город", 45_000),
    ("книга", 35_000),
    ("письмо", 35_000),
    ("вопрос", 50_000),
    ("ответ", 45_000),
    ("текст", 35_000),
    ("слово", 50_000),
    ("строка", 25_000),
    ("кнопка", 25_000),
    ("окно", 40_000),
    ("экран", 30_000),
];

/// Curated everyday English words and apostrophe contractions guaranteed to be
/// present in `en_freq.fst` (crucial for e.g. `дуеэы` -> `let's`).
const EN_CURATED: &[(&str, u64)] = &[
    ("hello", 95_000),
    ("world", 85_000),
    ("thanks", 80_000),
    ("please", 90_000),
    ("sorry", 85_000),
    ("let's", 110_000),
    ("don't", 150_000),
    ("doesn't", 90_000),
    ("didn't", 95_000),
    ("can't", 130_000),
    ("won't", 90_000),
    ("isn't", 85_000),
    ("aren't", 75_000),
    ("wasn't", 80_000),
    ("weren't", 60_000),
    ("haven't", 75_000),
    ("hasn't", 65_000),
    ("hadn't", 55_000),
    ("wouldn't", 75_000),
    ("couldn't", 75_000),
    ("shouldn't", 65_000),
    ("mustn't", 30_000),
    ("it's", 160_000),
    ("that's", 140_000),
    ("there's", 100_000),
    ("here's", 85_000),
    ("what's", 110_000),
    ("who's", 80_000),
    ("where's", 75_000),
    ("when's", 45_000),
    ("how's", 65_000),
    ("he's", 110_000),
    ("she's", 100_000),
    ("i'm", 180_000),
    ("i've", 110_000),
    ("i'll", 115_000),
    ("i'd", 95_000),
    ("you're", 135_000),
    ("you've", 90_000),
    ("you'll", 95_000),
    ("you'd", 75_000),
    ("we're", 105_000),
    ("we've", 80_000),
    ("we'll", 85_000),
    ("we'd", 60_000),
    ("they're", 100_000),
    ("they've", 70_000),
    ("they'll", 75_000),
    ("they'd", 55_000),
    ("it'll", 60_000),
    ("that'll", 45_000),
    ("y'all", 40_000),
    ("o'clock", 35_000),
    ("ma'am", 40_000),
    ("screenshot", 30_000),
    ("keyboard", 35_000),
    ("clipboard", 25_000),
    ("snippet", 25_000),
    ("settings", 45_000),
    ("dictionary", 30_000),
    ("layout", 35_000),
    ("offline", 30_000),
    ("terminal", 30_000),
    ("window", 55_000),
    ("desktop", 35_000),
];

fn is_valid_ru_word(word: &str) -> bool {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_ru_letter(first) {
        return false;
    }
    let mut len = 1usize;
    let mut hyphen_count = 0usize;
    for ch in chars {
        len += 1;
        if is_ru_letter(ch) {
            continue;
        }
        if ch == '-' && hyphen_count == 0 {
            hyphen_count += 1;
            continue;
        }
        return false;
    }
    if word.ends_with('-') {
        return false;
    }
    // Allow 1-letter valid Russian words (`я`, `в`, `и`, `к`, `о`, `с`, `у`, `а`) and 2..=32 char words
    if len == 1 {
        matches!(first, 'а' | 'в' | 'и' | 'к' | 'о' | 'с' | 'у' | 'я')
    } else {
        len <= 32
    }
}

/// Common typo/jargon/OCR noise present in OpenSubtitles that should not suppress layout/typo correction.
const EN_NOISE_EXCLUDE: &[&str] = &[
    "helo", "teh", "recieve", "definately", "seperate", "occured", "lf", "nen",
];

fn is_valid_en_word(word: &str) -> bool {
    if EN_NOISE_EXCLUDE.contains(&word) {
        return false;
    }
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_en_letter(first) {
        return false;
    }
    let mut len = 1usize;
    let mut punct_count = 0usize;
    for ch in chars {
        len += 1;
        if is_en_letter(ch) {
            continue;
        }
        if (ch == '\'' || ch == '-') && punct_count == 0 {
            punct_count += 1;
            continue;
        }
        return false;
    }
    if word.ends_with('\'') || word.ends_with('-') {
        return false;
    }
    if len == 1 {
        matches!(first, 'a' | 'i')
    } else {
        len <= 32
    }
}

fn is_ru_letter(ch: char) -> bool {
    ('а'..='я').contains(&ch) || ch == 'ё'
}

fn is_en_letter(ch: char) -> bool {
    ch.is_ascii_lowercase()
}

fn load_wordlist(
    path: &Path,
    curated: &[(&str, u64)],
    validator: fn(&str) -> bool,
    max_words: usize,
) -> Vec<(String, u64)> {
    let mut counts: HashMap<String, u64> = HashMap::with_capacity(max_words + curated.len());

    if path.exists() {
        let file = File::open(path).expect("failed to open wordlist");
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let mut parts = line.split_whitespace();
            let Some(raw_word) = parts.next() else {
                continue;
            };
            let freq = parts
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1);
            let word = raw_word.to_lowercase();
            if validator(&word) {
                counts
                    .entry(word)
                    .and_modify(|e| *e = (*e).max(freq))
                    .or_insert(freq);
            }
        }
    }

    for &(word, freq) in curated {
        counts
            .entry(word.to_string())
            .and_modify(|e| *e = (*e).max(freq))
            .or_insert(freq);
    }

    let mut items: Vec<(String, u64)> = counts.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items.truncate(max_words);
    items
}

fn build_fst(entries: &[(String, u64)], output: &Path) {
    // Map word -> rank (0 = most frequent), sorted lexicographically for FST insertion.
    let mut sorted: BTreeMap<&str, u64> = BTreeMap::new();
    for (rank, (word, _freq)) in entries.iter().enumerate() {
        sorted.insert(word.as_str(), rank as u64);
    }

    let wtr = BufWriter::new(File::create(output).expect("create fst file"));
    let mut builder = MapBuilder::new(wtr).expect("init fst builder");
    for (word, rank) in sorted {
        builder.insert(word, rank).expect("insert fst entry");
    }
    builder.finish().expect("finish fst");
}

/// Binary format for n-gram table:
/// - Magic: b"KCNG" (4 bytes)
/// - Version: u8 = 1
/// - Alphabet len: u8 (`N`, plus 1 boundary symbol `#` at index `N`, so `M = N + 1`)
/// - Bigram table: `M * M` i16 values (scaled log-probability * 100, little-endian)
/// - Trigram table: `M * M * M` i16 values (scaled log-probability * 100, little-endian)
fn build_ngrams(entries: &[(String, u64)], alphabet: &[char], output: &Path) {
    let n = alphabet.len();
    let m = n + 1; // index `n` is word boundary '^' / '$'
    let mut char_to_idx: HashMap<char, usize> = HashMap::with_capacity(n);
    for (i, &c) in alphabet.iter().enumerate() {
        char_to_idx.insert(c, i);
    }

    let mut bigram_counts = vec![0.0f64; m * m];
    let mut unigram_counts = vec![0.0f64; m];
    let mut trigram_counts = vec![0.0f64; m * m * m];
    let mut bigram_ctx_counts = vec![0.0f64; m * m];

    for (word, freq) in entries {
        let weight = ((*freq as f64) + 1.0).ln().max(1.0);
        let mut seq = Vec::with_capacity(word.len() + 2);
        seq.push(n); // start boundary
        for ch in word.chars() {
            if let Some(&idx) = char_to_idx.get(&ch) {
                seq.push(idx);
            }
        }
        seq.push(n); // end boundary

        for win in seq.windows(2) {
            let a = win[0];
            let b = win[1];
            bigram_counts[a * m + b] += weight;
            unigram_counts[a] += weight;
        }
        for win in seq.windows(3) {
            let a = win[0];
            let b = win[1];
            let c = win[2];
            trigram_counts[(a * m + b) * m + c] += weight;
            bigram_ctx_counts[a * m + b] += weight;
        }
    }

    let smooth_bi = 0.5f64;
    let smooth_tri = 0.25f64;

    let mut out = BufWriter::new(File::create(output).expect("create ngram file"));
    out.write_all(b"KCNG").expect("write magic");
    out.write_all(&[1u8, n as u8]).expect("write header");

    for a in 0..m {
        let denom = unigram_counts[a] + smooth_bi * (m as f64);
        for b in 0..m {
            let p = (bigram_counts[a * m + b] + smooth_bi) / denom;
            let score = (p.ln() * 100.0).round().clamp(-32000.0, 0.0) as i16;
            out.write_all(&score.to_le_bytes()).expect("write bigram");
        }
    }

    for a in 0..m {
        for b in 0..m {
            let denom = bigram_ctx_counts[a * m + b] + smooth_tri * (m as f64);
            for c in 0..m {
                let p = (trigram_counts[(a * m + b) * m + c] + smooth_tri) / denom;
                let score = (p.ln() * 100.0).round().clamp(-32000.0, 0.0) as i16;
                out.write_all(&score.to_le_bytes()).expect("write trigram");
            }
        }
    }

    out.flush().expect("flush ngram file");
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    let wordlists_dir = root.join("data").join("wordlists");
    let assets_dir = root.join("crates").join("core-engine").join("assets");
    fs::create_dir_all(&assets_dir).expect("create assets dir");

    let ru_entries = load_wordlist(
        &wordlists_dir.join("ru_50k_raw.txt"),
        RU_CURATED,
        is_valid_ru_word,
        50_000,
    );
    let en_entries = load_wordlist(
        &wordlists_dir.join("en_50k_raw.txt"),
        EN_CURATED,
        is_valid_en_word,
        50_000,
    );

    println!(
        "Loaded {} RU words and {} EN words",
        ru_entries.len(),
        en_entries.len()
    );

    build_fst(&ru_entries, &assets_dir.join("ru_freq.fst"));
    build_fst(&en_entries, &assets_dir.join("en_freq.fst"));
    build_ngrams(&ru_entries, RU_ALPHABET, &assets_dir.join("ru_ngrams.bin"));
    build_ngrams(&en_entries, EN_ALPHABET, &assets_dir.join("en_ngrams.bin"));

    for name in [
        "ru_freq.fst",
        "en_freq.fst",
        "ru_ngrams.bin",
        "en_ngrams.bin",
    ] {
        let meta = fs::metadata(assets_dir.join(name)).expect("stat asset");
        println!("  {} -> {} bytes", name, meta.len());
    }
}
