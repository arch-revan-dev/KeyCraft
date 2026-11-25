# Wordlist Licenses & Provenance

The offline frequency wordlists in this directory are compiled at build time into compact binary FST and n-gram indices embedded into `crates/core-engine/assets/`. **KeyCraft performs zero network requests at runtime.**

## Sources
- **FrequencyWords (`ru_50k_raw.txt`, `en_50k_raw.txt`)**: Derived from OpenSubtitles 2018 frequency counts curated by Hermit Dave (`https://github.com/hermitdave/FrequencyWords`), licensed under **MIT** (code) and **CC-BY-SA 4.0** (frequency wordlists).
- **Curated Supplement (`ru_extra.txt`, `en_extra.txt`)**: Common Russian words containing `ё`, everyday IT/desktop terms, and standard English apostrophe contractions (`let's`, `don't`, `it's`, `can't`, `won't`, etc.), authored for KeyCraft under the **MIT License**.
