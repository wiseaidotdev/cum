// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use cum_rs::stochastic::{LanguageHint, StochasticEnhancer, SynonymBank, detect_language};

#[test]
fn test_language_hint_bcp47_mappings() {
    assert_eq!(LanguageHint::English.as_bcp47(), "en");
    assert_eq!(LanguageHint::Spanish.as_bcp47(), "es");
    assert_eq!(LanguageHint::Arabic.as_bcp47(), "ar");
    assert_eq!(LanguageHint::French.as_bcp47(), "fr");
    assert_eq!(LanguageHint::German.as_bcp47(), "de");
    assert_eq!(LanguageHint::Auto.as_bcp47(), "auto");
}

#[test]
fn test_language_detection_arabic() {
    let text = "يبدأ النص يحتوي على كلمات مخفية وعلامات غير مرئية في النص العربي";
    assert_eq!(detect_language(text), LanguageHint::Arabic);
}

#[test]
fn test_language_detection_spanish() {
    let text = "El texto comienza con un formato especial que contiene señales invisibles";
    assert_eq!(detect_language(text), LanguageHint::Spanish);
}

#[test]
fn test_language_detection_fallback_english() {
    let text = "This text begins with a UTF-8 BOM and contains bidirectional format controls.";
    assert_eq!(detect_language(text), LanguageHint::English);
}

#[test]
fn test_spanish_enhancement_works() {
    let e = StochasticEnhancer::with_language_and_probability(LanguageHint::Spanish, 1.0);

    assert_eq!(e.enhance("test").language, LanguageHint::Spanish);

    let out = e.enhance("texto formato");
    assert!(
        out.words_substituted > 0,
        "Spanish words should be substituted"
    );
}

#[test]
fn test_french_enhancement_works() {
    let e = StochasticEnhancer::with_language_and_probability(LanguageHint::French, 1.0);
    assert_eq!(e.enhance("test").language, LanguageHint::French);

    let out = e.enhance("complexe fichier");
    assert!(
        out.words_substituted > 0,
        "French words should be substituted"
    );
}

#[test]
fn test_german_enhancement_works() {
    let e = StochasticEnhancer::with_language_and_probability(LanguageHint::German, 1.0);
    assert_eq!(e.enhance("test").language, LanguageHint::German);

    let out = e.enhance("text format");
    assert!(
        out.words_substituted > 0,
        "German words should be substituted"
    );
}

#[test]
fn test_arabic_enhancement_works() {
    let e = StochasticEnhancer::with_language_and_probability(LanguageHint::Arabic, 1.0);
    assert_eq!(e.enhance("test").language, LanguageHint::Arabic);

    let out = e.enhance("نص");
    assert!(
        out.words_substituted > 0,
        "Arabic words should be substituted"
    );
}

#[test]
fn test_synonym_bank_curated_language_counts() {
    assert!(SynonymBank::with_language(LanguageHint::English).curated_count() > 50);
    assert!(SynonymBank::with_language(LanguageHint::Spanish).curated_count() > 10);
    assert!(SynonymBank::with_language(LanguageHint::French).curated_count() > 10);
    assert!(SynonymBank::with_language(LanguageHint::German).curated_count() > 10);
    assert!(SynonymBank::with_language(LanguageHint::Arabic).curated_count() > 10);
}
