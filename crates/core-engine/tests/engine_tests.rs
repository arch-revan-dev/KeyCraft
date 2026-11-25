//! Comprehensive unit test suite for `keycraft-core-engine` (65+ deterministic test cases).

use std::time::Instant;

use keycraft_core_engine::config::{
    AppRule, KeyCraftConfig, Language, PinnedLayout, SnippetActivation, SnippetEntry, WordSource,
};
use keycraft_core_engine::guards::{check_word_guards, GuardVerdict, SkipReason};
use keycraft_core_engine::keymap::{convert_en_to_ru, convert_ru_to_en, swap_special_symbol};
use keycraft_core_engine::lists_case::{cycle_case, evaluate_smart_list_enter, SmartListAction};
use keycraft_core_engine::{ActiveAppState, EngineAction, EngineInput, KeyCraftEngine};

fn default_app() -> ActiveAppState {
    ActiveAppState {
        process_name: "notepad.exe".to_string(),
        is_password_field: false,
    }
}

fn type_word_and_space(engine: &mut KeyCraftEngine, word: &str, layout: Language) -> EngineAction {
    let app = default_app();
    for ch in word.chars() {
        let act = engine.process(EngineInput::Char { ch, layout }, &app);
        if act != EngineAction::NoOp {
            return act;
        }
    }
    engine.process(EngineInput::Space { layout }, &app)
}

fn assert_replaced(action: EngineAction, expected_text: &str) {
    match action {
        EngineAction::ReplaceText { text, .. } => assert_eq!(text, expected_text),
        other => panic!("expected ReplaceText({expected_text:?}), got {other:?}"),
    }
}

// =========================================================================
// 1. AUTO LAYOUT SWITCHING (RU <-> EN), CASING, CAPSLOCK, APOSTROPHES, `Ё`
// =========================================================================

#[test]
fn t01_en_to_ru_privet() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
}

#[test]
fn t02_ru_to_en_hello() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "руддщ", Language::Ru), "hello ");
}

#[test]
fn t03_ru_to_en_contraction_lets() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "дуеэы", Language::Ru), "let's ");
}

#[test]
fn t04_ru_to_en_contraction_dont() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "вщтэе", Language::Ru), "don't ");
}

#[test]
fn t05_ru_to_en_contraction_its() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "шеэы", Language::Ru), "it's ");
}

#[test]
fn t06_preserve_capitalized_case() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "Ghbdtn", Language::En), "Привет ");
}

#[test]
fn t07_preserve_all_uppercase() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "GHBDTN", Language::En), "ПРИВЕТ ");
}

#[test]
fn t08_preserve_ru_to_en_capitalized() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "Руддщ", Language::Ru), "Hello ");
}

#[test]
fn t09_fix_accidental_capslock_ru() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "пРИВЕТ", Language::Ru), "Привет ");
}

#[test]
fn t10_fix_accidental_capslock_en() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "hELLO", Language::En), "Hello ");
}

#[test]
fn t11_fix_wrong_layout_plus_accidental_capslock() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "gHBDTN", Language::En), "Привет ");
}

#[test]
fn t12_en_punctuation_mapping_to_ru_lyublyu() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "k.,k.", Language::En), "люблю ");
}

#[test]
fn t13_en_bracket_mapping_to_ru_khorosho() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "[jhjij", Language::En), "хорошо ");
}

#[test]
fn t14_en_backtick_mapping_to_ru_yo_esche() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "to`", Language::En), "ещё ");
}

#[test]
fn t15_en_quote_mapping_to_ru_eto() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "'nj", Language::En), "это ");
}

#[test]
fn t16_preserve_trailing_punctuation_on_switch() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "ghbdtn,", Language::En), "привет, ");
}

#[test]
fn t17_preserve_trailing_exclamation_on_switch() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "руддщ!", Language::Ru), "hello! ");
}

#[test]
fn t18_valid_ru_word_not_falsely_switched() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_eq!(type_word_and_space(&mut eng, "привет", Language::Ru), EngineAction::NoOp);
}

#[test]
fn t19_valid_en_word_not_falsely_switched() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_eq!(type_word_and_space(&mut eng, "hello", Language::En), EngineAction::NoOp);
}

#[test]
fn t20_valid_ru_with_yo_not_falsely_switched() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_eq!(type_word_and_space(&mut eng, "всё", Language::Ru), EngineAction::NoOp);
}

#[test]
fn t21_switch_emits_os_layout_change_when_enabled() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    match type_word_and_space(&mut eng, "ghbdtn", Language::En) {
        EngineAction::ReplaceText { switch_layout, .. } => assert_eq!(switch_layout, Some(Language::Ru)),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn t22_switch_respects_disabled_os_layout_flag() {
    let mut cfg = KeyCraftConfig::default();
    cfg.layout.switch_system_layout = false;
    let mut eng = KeyCraftEngine::new(cfg);
    match type_word_and_space(&mut eng, "ghbdtn", Language::En) {
        EngineAction::ReplaceText { switch_layout, text, .. } => {
            assert_eq!(text, "привет ");
            assert_eq!(switch_layout, None);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn t23_ngram_fallback_switches_long_unlisted_ru_word() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    // "перекомпилировать" typed on EN layout
    let typed = convert_ru_to_en("перекомпилировать");
    assert_replaced(
        type_word_and_space(&mut eng, &typed, Language::En),
        "перекомпилировать ",
    );
}

#[test]
fn t24_capslock_disabled_leaves_word_untouched() {
    let mut cfg = KeyCraftConfig::default();
    cfg.layout.fix_capslock = false;
    let mut eng = KeyCraftEngine::new(cfg);
    assert_eq!(type_word_and_space(&mut eng, "пРИВЕТ", Language::Ru), EngineAction::NoOp);
}

// =========================================================================
// 2. FALSE-POSITIVE GUARDS (URL, PATHS, EMAIL, CODE, DIGITS, MIXED)
// =========================================================================

#[test]
fn t25_guard_https_url() {
    assert_eq!(
        check_word_guards("https://keyray.ru/docs"),
        GuardVerdict::Skip(SkipReason::UrlOrDomain)
    );
}

#[test]
fn t26_guard_www_domain() {
    assert_eq!(
        check_word_guards("www.example.com"),
        GuardVerdict::Skip(SkipReason::UrlOrDomain)
    );
}

#[test]
fn t27_guard_bare_domain() {
    assert_eq!(
        check_word_guards("github.com"),
        GuardVerdict::Skip(SkipReason::UrlOrDomain)
    );
}

#[test]
fn t28_guard_windows_path() {
    assert_eq!(
        check_word_guards(r"C:\Users\kairos\file.txt"),
        GuardVerdict::Skip(SkipReason::FilePath)
    );
}

#[test]
fn t29_guard_unix_path() {
    assert_eq!(
        check_word_guards("/usr/local/bin"),
        GuardVerdict::Skip(SkipReason::FilePath)
    );
}

#[test]
fn t30_guard_email() {
    assert_eq!(
        check_word_guards("dev@keycraft.org"),
        GuardVerdict::Skip(SkipReason::EmailAddress)
    );
}

#[test]
fn t31_guard_camel_case() {
    assert_eq!(
        check_word_guards("camelCaseVar"),
        GuardVerdict::Skip(SkipReason::CodeIdentifier)
    );
}

#[test]
fn t32_guard_pascal_case() {
    assert_eq!(
        check_word_guards("HttpClient"),
        GuardVerdict::Skip(SkipReason::CodeIdentifier)
    );
}

#[test]
fn t33_guard_snake_case() {
    assert_eq!(
        check_word_guards("snake_case_id"),
        GuardVerdict::Skip(SkipReason::CodeIdentifier)
    );
}

#[test]
fn t34_guard_rust_path_colons() {
    assert_eq!(
        check_word_guards("std::vec"),
        GuardVerdict::Skip(SkipReason::CodeIdentifier)
    );
}

#[test]
fn t35_guard_digits_in_word() {
    assert_eq!(
        check_word_guards("utf8"),
        GuardVerdict::Skip(SkipReason::ContainsDigits)
    );
}

#[test]
fn t36_guard_cli_flag() {
    assert_eq!(
        check_word_guards("--release"),
        GuardVerdict::Skip(SkipReason::CliFlag)
    );
}

#[test]
fn t37_guard_mixed_scripts() {
    assert_eq!(
        check_word_guards("приветWorld"),
        GuardVerdict::Skip(SkipReason::MixedScripts)
    );
}

#[test]
fn t38_engine_never_mutates_url_or_code() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_eq!(type_word_and_space(&mut eng, "https://example.org", Language::En), EngineAction::NoOp);
    assert_eq!(type_word_and_space(&mut eng, "myVarName", Language::En), EngineAction::NoOp);
    assert_eq!(type_word_and_space(&mut eng, "foo_bar", Language::En), EngineAction::NoOp);
    assert_eq!(type_word_and_space(&mut eng, r"C:\Windows", Language::En), EngineAction::NoOp);
}

// =========================================================================
// 3. SINGLE SPECIAL SYMBOL SWAP, UNDO, AUTO-LEARN, USER DICTIONARY
// =========================================================================

#[test]
fn t39_swap_single_symbol_numero_to_hash() {
    assert_eq!(swap_special_symbol('№', Language::Ru), Some('#'));
    assert_eq!(swap_special_symbol('#', Language::En), Some('№'));
}

#[test]
fn t40_swap_single_symbol_quote_and_semicolon() {
    assert_eq!(swap_special_symbol('"', Language::Ru), Some('@'));
    assert_eq!(swap_special_symbol(';', Language::Ru), Some('$'));
    assert_eq!(swap_special_symbol('@', Language::En), Some('"'));
    assert_eq!(swap_special_symbol('$', Language::En), Some(';'));
}

#[test]
fn t41_engine_swap_last_symbol_action() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let app = default_app();
    eng.process(EngineInput::Char { ch: '№', layout: Language::Ru }, &app);
    assert_replaced(
        eng.process(EngineInput::SwapLastSymbol { layout: Language::Ru }, &app),
        "#",
    );
}

#[test]
fn t42_undo_reverts_last_auto_switch() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let app = default_app();
    assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
    match eng.process(EngineInput::UndoLast, &app) {
        EngineAction::ReplaceText {
            backspace_count,
            text,
            switch_layout,
            ..
        } => {
            assert_eq!(backspace_count, "привет ".chars().count());
            assert_eq!(text, "ghbdtn ");
            assert_eq!(switch_layout, Some(Language::En));
        }
        other => panic!("expected undo ReplaceText, got {other:?}"),
    }
}

#[test]
fn t43_auto_learn_after_threshold_cancellations() {
    let mut cfg = KeyCraftConfig::default();
    cfg.layout.auto_learn_threshold = 3;
    let mut eng = KeyCraftEngine::new(cfg);
    let app = default_app();

    for i in 1..=3 {
        assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
        match eng.process(EngineInput::UndoLast, &app) {
            EngineAction::ReplaceText { auto_learned_word, .. } => {
                if i < 3 {
                    assert_eq!(auto_learned_word, None);
                } else {
                    assert_eq!(auto_learned_word.as_deref(), Some("ghbdtn"));
                }
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    // Now `ghbdtn` is protected in the whitelist and will NOT auto-switch again!
    assert_eq!(type_word_and_space(&mut eng, "ghbdtn", Language::En), EngineAction::NoOp);
}

#[test]
fn t44_user_dictionary_protects_and_allows_target() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    eng.add_user_word("keycraft", WordSource::Manual);
    // Protected when typed as `keycraft`
    assert_eq!(type_word_and_space(&mut eng, "keycraft", Language::En), EngineAction::NoOp);
    // Recognized as valid EN target when typed on RU layout (`лунскфае`)
    let typed_ru = convert_en_to_ru("keycraft");
    assert_replaced(type_word_and_space(&mut eng, &typed_ru, Language::Ru), "keycraft ");
}

// =========================================================================
// 4. DETERMINISTIC TYPO CORRECTION (DL = 1)
// =========================================================================

#[test]
fn t45_typo_ru_transposition() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "привте", Language::Ru), "привет ");
}

#[test]
fn t46_typo_ru_extra_double_letter() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "приввет", Language::Ru), "привет ");
}

#[test]
fn t47_typo_ru_adjacent_key_miss() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    // `н` is directly adjacent to `е` on ЙЦУКЕН
    assert_replaced(type_word_and_space(&mut eng, "привнт", Language::Ru), "привет ");
}

#[test]
fn t48_typo_en_transposition_teh() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "teh", Language::En), "the ");
}

#[test]
fn t49_typo_en_omitted_letter_hllo() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "hllo", Language::En), "hello ");
}

#[test]
fn t50_typo_en_preserves_capitalization() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "Hllo", Language::En), "Hello ");
}

#[test]
fn t68_typo_ru_omitted_letter_programma() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "прогрмма", Language::Ru), "программа ");
}

#[test]
fn t69_ye_lf_nen_switch_to_nu_da_tut() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "ye", Language::En), "ну ");
    assert_replaced(type_word_and_space(&mut eng, "lf", Language::En), "да ");
    assert_replaced(type_word_and_space(&mut eng, "nen", Language::En), "тут ");
}

#[test]
fn t70_ds_vs_switch_to_vy_my() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "ds", Language::En), "вы ");
    assert_replaced(type_word_and_space(&mut eng, "vs", Language::En), "мы ");
}

#[test]
fn t71_common_en_short_words_never_falsely_switch() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    for word in ["no", "ok", "to", "in", "on", "at", "he", "it", "yes", "how", "are", "you"] {
        assert_eq!(
            type_word_and_space(&mut eng, word, Language::En),
            EngineAction::NoOp,
            "common EN word `{word}` was falsely switched"
        );
    }
}

#[test]
fn t72_typo_ru_vowel_confusions() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "сматришь", Language::Ru), "смотришь ");
    assert_replaced(type_word_and_space(&mut eng, "чта", Language::Ru), "что ");
    assert_replaced(type_word_and_space(&mut eng, "дила", Language::Ru), "дела ");
}

#[test]
fn t73_two_letter_ru_abbrevs_never_switch() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    for abbr in ["пк", "тк", "вк", "тг", "лс", "хз", "др", "тд"] {
        assert_eq!(
            type_word_and_space(&mut eng, abbr, Language::Ru),
            EngineAction::NoOp,
            "RU abbreviation `{abbr}` should not auto-switch"
        );
    }
}

#[test]
fn t74_sticky_shift_chtoby_privet_hello() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "ЧТОбы", Language::Ru), "Чтобы ");
    assert_replaced(type_word_and_space(&mut eng, "ПРивет", Language::Ru), "Привет ");
    assert_replaced(type_word_and_space(&mut eng, "HEllo", Language::En), "Hello ");
}

#[test]
fn t75_sticky_shift_ignores_acronyms_and_pascal_code() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_eq!(type_word_and_space(&mut eng, "США", Language::Ru), EngineAction::NoOp);
    assert_eq!(type_word_and_space(&mut eng, "IPAddress", Language::En), EngineAction::NoOp);
    assert_eq!(type_word_and_space(&mut eng, "HttpClient", Language::En), EngineAction::NoOp);
}

#[test]
fn t76_screenshot_enabled_serde_default() {
    let cfg = KeyCraftConfig::default();
    assert!(cfg.screenshot.enabled);
    assert!(!cfg.layout.undo_on_backspace);
}

#[test]
fn t77_undo_on_double_backspace_restores_word() {
    let mut cfg = KeyCraftConfig::default();
    cfg.layout.undo_on_backspace = true;
    let mut eng = KeyCraftEngine::new(cfg);
    let app = default_app();

    assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
    // Backspace #1 removes the trailing space (flush against `привет|`) -> NoOp
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
    // Backspace #2 while flush against `привет` cancels replacement (OS already deleted 1 char -> erase 5 remaining)
    match eng.process(EngineInput::Backspace, &app) {
        EngineAction::ReplaceText {
            backspace_count,
            text,
            switch_layout,
            ..
        } => {
            assert_eq!(backspace_count, "привет".chars().count() - 1);
            assert_eq!(text, "ghbdtn");
            assert_eq!(switch_layout, Some(Language::En));
        }
        other => panic!("expected backspace undo ReplaceText, got {other:?}"),
    }
}

#[test]
fn t78_undo_on_backspace_disabled_by_default() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let app = default_app();
    assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
}

#[test]
fn t79_undo_on_double_backspace_triggers_auto_learn() {
    let mut cfg = KeyCraftConfig::default();
    cfg.layout.undo_on_backspace = true;
    cfg.layout.auto_learn_threshold = 2;
    let mut eng = KeyCraftEngine::new(cfg);
    let app = default_app();

    // Cancel #1
    assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
    match eng.process(EngineInput::Backspace, &app) {
        EngineAction::ReplaceText { auto_learned_word, .. } => assert_eq!(auto_learned_word, None),
        other => panic!("unexpected {other:?}"),
    }

    // Reset and Cancel #2 -> should auto-learn `ghbdtn`
    eng.reset_buffer();
    assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
    match eng.process(EngineInput::Backspace, &app) {
        EngineAction::ReplaceText { auto_learned_word, .. } => {
            assert_eq!(auto_learned_word.as_deref(), Some("ghbdtn"));
        }
        other => panic!("unexpected {other:?}"),
    }
    eng.reset_buffer();
    assert_eq!(type_word_and_space(&mut eng, "ghbdtn", Language::En), EngineAction::NoOp);
}

#[test]
fn t80_undo_on_double_backspace_works_for_typos_and_clears_on_typing() {
    let mut cfg = KeyCraftConfig::default();
    cfg.layout.undo_on_backspace = true;
    let mut eng = KeyCraftEngine::new(cfg);
    let app = default_app();

    // Typo fix `привте` -> `привет ` reverts on double-Backspace
    assert_replaced(type_word_and_space(&mut eng, "привте", Language::Ru), "привет ");
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
    assert_replaced(eng.process(EngineInput::Backspace, &app), "привте");

    // Typing a new character after Backspace #1 invalidates pending Backspace undo
    eng.reset_buffer();
    assert_replaced(type_word_and_space(&mut eng, "ghbdtn", Language::En), "привет ");
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
    assert_eq!(eng.process(EngineInput::Char { ch: 'а', layout: Language::Ru }, &app), EngineAction::NoOp);
    assert_eq!(eng.process(EngineInput::Backspace, &app), EngineAction::NoOp);
}

#[test]
fn t51_typo_can_be_disabled() {
    let mut cfg = KeyCraftConfig::default();
    cfg.typos.enabled = false;
    let mut eng = KeyCraftEngine::new(cfg);
    assert_eq!(type_word_and_space(&mut eng, "привте", Language::Ru), EngineAction::NoOp);
}

#[test]
fn t52_typo_undo_restores_original() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let app = default_app();
    assert_replaced(type_word_and_space(&mut eng, "привте", Language::Ru), "привет ");
    assert_replaced(eng.process(EngineInput::UndoLast, &app), "привте ");
}

// =========================================================================
// 5. SNIPPETS & CROSS-LAYOUT TEXT EXPANDER
// =========================================================================

#[test]
fn t53_snippet_ruble_multi_trigger() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "р-", Language::Ru), "₽ ");
    assert_replaced(type_word_and_space(&mut eng, "Р-", Language::Ru), "₽ ");
}

#[test]
fn t54_snippet_em_dash_and_arrow_on_space() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_replaced(type_word_and_space(&mut eng, "---", Language::En), "— ");
    assert_replaced(type_word_and_space(&mut eng, "->", Language::En), "→ ");
    assert_replaced(type_word_and_space(&mut eng, "+-", Language::En), "± ");
}

#[test]
fn t55_snippet_instant_guillemets_and_copyright() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let app = default_app();
    assert_eq!(eng.process(EngineInput::Char { ch: '<', layout: Language::En }, &app), EngineAction::NoOp);
    assert_replaced(eng.process(EngineInput::Char { ch: '<', layout: Language::En }, &app), "«");

    assert_eq!(eng.process(EngineInput::Char { ch: '>', layout: Language::En }, &app), EngineAction::NoOp);
    assert_replaced(eng.process(EngineInput::Char { ch: '>', layout: Language::En }, &app), "»");

    eng.process(EngineInput::Char { ch: '(', layout: Language::En }, &app);
    eng.process(EngineInput::Char { ch: 'c', layout: Language::En }, &app);
    assert_replaced(eng.process(EngineInput::Char { ch: ')', layout: Language::En }, &app), "©");
}

#[test]
fn t56_snippet_cross_layout_any_layout() {
    let mut cfg = KeyCraftConfig::default();
    cfg.snippets.push(SnippetEntry {
        id: "admin".to_string(),
        triggers: vec!["+админ".to_string()],
        replacement: "admin@company.local".to_string(),
        activation: SnippetActivation::Space,
        any_layout: true,
        enabled: true,
    });
    let mut eng = KeyCraftEngine::new(cfg);
    // Typed `+flvby` on EN layout instead of `+админ`
    assert_replaced(
        type_word_and_space(&mut eng, "+flvby", Language::En),
        "admin@company.local ",
    );
}

#[test]
fn t57_snippet_tab_activation() {
    let mut cfg = KeyCraftConfig::default();
    cfg.snippets.push(SnippetEntry {
        id: "sig".to_string(),
        triggers: vec!["sig".to_string()],
        replacement: "С уважением, Команда".to_string(),
        activation: SnippetActivation::Tab,
        any_layout: false,
        enabled: true,
    });
    let mut eng = KeyCraftEngine::new(cfg);
    let app = default_app();
    for ch in "sig".chars() {
        eng.process(EngineInput::Char { ch, layout: Language::En }, &app);
    }
    assert_replaced(
        eng.process(EngineInput::Tab { layout: Language::En }, &app),
        "С уважением, Команда",
    );
}

// =========================================================================
// 6. CASE CYCLING, AUTO-CAPITALIZATION & SMART LISTS
// =========================================================================

#[test]
fn t58_cycle_case_four_states() {
    let s1 = cycle_case("привет мир");
    assert_eq!(s1, "Привет мир");
    let s2 = cycle_case(&s1);
    assert_eq!(s2, "ПРИВЕТ МИР");
    let s3 = cycle_case(&s2);
    assert_eq!(s3, "Привет Мир");
    let s4 = cycle_case(&s3);
    assert_eq!(s4, "привет мир");
}

#[test]
fn t59_auto_capitalize_after_sentence_punctuation() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    assert_eq!(type_word_and_space(&mut eng, "привет.", Language::Ru), EngineAction::NoOp);
    assert_replaced(type_word_and_space(&mut eng, "мир", Language::Ru), "Мир ");
}

#[test]
fn t60_smart_list_numbered_and_exit() {
    assert_eq!(
        evaluate_smart_list_enter("1. Первый пункт"),
        Some(SmartListAction::InsertNextMarker("2. ".to_string()))
    );
    assert_eq!(
        evaluate_smart_list_enter("  12) Подпункт"),
        Some(SmartListAction::InsertNextMarker("  13) ".to_string()))
    );
    assert_eq!(
        evaluate_smart_list_enter("2. "),
        Some(SmartListAction::ExitEmptyItem { erase_chars: 3 })
    );
}

#[test]
fn t61_smart_list_bulleted_and_exit() {
    assert_eq!(
        evaluate_smart_list_enter("- Задача"),
        Some(SmartListAction::InsertNextMarker("- ".to_string()))
    );
    assert_eq!(
        evaluate_smart_list_enter("• Пункт"),
        Some(SmartListAction::InsertNextMarker("• ".to_string()))
    );
    assert_eq!(
        evaluate_smart_list_enter("* Элемент"),
        Some(SmartListAction::InsertNextMarker("* ".to_string()))
    );
    assert_eq!(
        evaluate_smart_list_enter("- "),
        Some(SmartListAction::ExitEmptyItem { erase_chars: 2 })
    );
}

// =========================================================================
// 7. ZERO-KEYLOGGER SECURITY, APP RULES, MANUAL HOTKEY, LATENCY BUDGET
// =========================================================================

#[test]
fn t62_password_field_bypasses_and_zeroizes() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let pwd_app = ActiveAppState {
        process_name: "chrome.exe".to_string(),
        is_password_field: true,
    };
    for ch in "ghbdtn".chars() {
        assert_eq!(
            eng.process(EngineInput::Char { ch, layout: Language::En }, &pwd_app),
            EngineAction::NoOp
        );
    }
    assert_eq!(
        eng.process(EngineInput::Space { layout: Language::En }, &pwd_app),
        EngineAction::NoOp
    );
}

#[test]
fn t63_per_app_exclusion_blocks_terminal() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let term = ActiveAppState {
        process_name: "WindowsTerminal.exe".to_string(),
        is_password_field: false,
    };
    for ch in "ghbdtn".chars() {
        eng.process(EngineInput::Char { ch, layout: Language::En }, &term);
    }
    assert_eq!(
        eng.process(EngineInput::Space { layout: Language::En }, &term),
        EngineAction::NoOp
    );
}

#[test]
fn t64_per_app_granular_ide_allows_snippets_blocks_autoswitch() {
    let mut cfg = KeyCraftConfig::default();
    cfg.app_rules.push(AppRule::ide_preset("CustomIde.exe", "Custom IDE"));
    let mut eng = KeyCraftEngine::new(cfg);
    let ide = ActiveAppState {
        process_name: "CustomIde.exe".to_string(),
        is_password_field: false,
    };

    // Layout auto-switch disabled in IDE
    for ch in "ghbdtn".chars() {
        eng.process(EngineInput::Char { ch, layout: Language::En }, &ide);
    }
    assert_eq!(eng.process(EngineInput::Space { layout: Language::En }, &ide), EngineAction::NoOp);

    // Snippets still allowed in IDE preset
    for ch in "->".chars() {
        eng.process(EngineInput::Char { ch, layout: Language::En }, &ide);
    }
    assert_replaced(eng.process(EngineInput::Space { layout: Language::En }, &ide), "→ ");
}

#[test]
fn t65_manual_convert_selection_and_last_word() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let (converted, lang) = eng.convert_selected_text("Ghbdtn vbh!");
    assert_eq!(converted, "Привет мир!");
    assert_eq!(lang, Language::Ru);

    let app = default_app();
    for ch in "ghbdtn".chars() {
        eng.process(EngineInput::Char { ch, layout: Language::En }, &app);
    }
    assert_replaced(eng.process(EngineInput::ManualConvertLastWord, &app), "привет");
}

#[test]
fn t66_config_json_roundtrip_and_pinned_layout() {
    let cfg = KeyCraftConfig::default();
    let json = serde_json::to_string_pretty(&cfg).expect("serialize config");
    let decoded: KeyCraftConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(cfg, decoded);
    assert_eq!(
        decoded.find_app_rule("WindowsTerminal.exe").map(|r| r.pinned_layout),
        Some(PinnedLayout::En)
    );
}

#[test]
fn t67_latency_under_2ms_per_word() {
    let mut eng = KeyCraftEngine::new(KeyCraftConfig::default());
    let words = ["ghbdtn", "руддщ", "привте", "helo", "https://example.com", "привет", "дуеэы"];
    let start = Instant::now();
    let iterations = 100;
    for _ in 0..iterations {
        for &w in &words {
            let _ = type_word_and_space(&mut eng, w, Language::En);
        }
    }
    let total_words = (iterations * words.len()) as u128;
    let avg_us = start.elapsed().as_micros() / total_words;
    assert!(
        avg_us < 2_000,
        "average word evaluation took {avg_us} µs, exceeding 2000 µs (2 ms) budget"
    );
}
