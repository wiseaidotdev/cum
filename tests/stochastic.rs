// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for [`cum_rs::stochastic`].
//!
//! These tests exercise [`SynonymBank`], [`StochasticEnhancer`], and
//! helper functions through the public API without mocking the RNG: the
//! test inputs are chosen so that the assertions hold regardless of the
//! specific random choices made by the engine.

use cum_rs::stochastic::{StochasticEnhancer, SynonymBank, capitalize, is_stop_word, split_token};

#[test]
fn test_enhance_empty_string() {
    let out = StochasticEnhancer::new(1.0).enhance("");
    assert!(out.text.is_empty());
    assert_eq!(out.words_substituted, 0);
}

#[test]
fn test_enhance_probability_zero_leaves_input_unchanged() {
    let input = "The chaos governs the universe and order.";
    let out = StochasticEnhancer::new(0.0).enhance(input);
    assert_eq!(out.text, input);
    assert_eq!(out.words_substituted, 0);
    assert_eq!(out.probability, 0.0);
}

#[test]
fn test_enhance_probability_one_substitutes_curated_word() {
    let out = StochasticEnhancer::new(1.0).enhance("chaos");
    assert_ne!(
        out.text, "chaos",
        "p=1.0 must substitute the curated word \"chaos\""
    );
    assert_eq!(out.words_substituted, 1);
}

#[test]
fn test_enhance_preserves_line_count() {
    let input = "chaos\nenergy\npattern";
    let out = StochasticEnhancer::new(1.0).enhance(input);
    assert_eq!(
        out.text.lines().count(),
        3,
        "line count must be preserved after enhancement"
    );
}

#[test]
fn test_enhance_stop_words_are_never_substituted() {
    let stop_words = ["the", "and", "or", "a", "in", "is"];
    for stop in &stop_words {
        let out = StochasticEnhancer::new(1.0).enhance(stop);
        assert_eq!(
            out.text.to_lowercase(),
            *stop,
            "stop word \"{stop}\" must not be substituted"
        );
        assert_eq!(out.words_substituted, 0);
    }
}

#[test]
fn test_enhance_output_carries_correct_probability() {
    let e = StochasticEnhancer::new(0.42);
    let out = e.enhance("chaos governs");
    assert_eq!(out.probability, 0.42);
}

#[test]
fn test_enhancer_clamps_probability() {
    assert_eq!(StochasticEnhancer::new(-1.0).probability(), 0.0);
    assert_eq!(StochasticEnhancer::new(2.0).probability(), 1.0);
    assert_eq!(StochasticEnhancer::new(0.75).probability(), 0.75);
}

#[test]
fn test_synonym_bank_curated_count_is_substantial() {
    assert!(
        SynonymBank::new().curated_count() > 50,
        "curated synonym count must exceed 50"
    );
}

#[test]
fn test_synonym_bank_candidate_finds_curated_entries() {
    let bank = SynonymBank::new();
    let mut rng = rand::rng();
    let curated_words = ["chaos", "energy", "pattern", "governs", "reveals"];
    for word in &curated_words {
        assert!(
            bank.candidate(word, &mut rng).is_some(),
            "expected a candidate for curated word \"{word}\""
        );
    }
}

#[test]
fn test_capitalize_helper() {
    assert_eq!(capitalize("hello"), "Hello");
    assert_eq!(capitalize("WORLD"), "WORLD");
    assert_eq!(capitalize("a"), "A");
    assert_eq!(capitalize(""), "");
    assert_eq!(capitalize("already"), "Already");
}

#[test]
fn test_is_stop_word_membership() {
    assert!(is_stop_word("the"));
    assert!(is_stop_word("and"));
    assert!(is_stop_word("a"));
    assert!(is_stop_word("every"));
    assert!(!is_stop_word("chaos"));
    assert!(!is_stop_word("entropy"));
    assert!(!is_stop_word("universe"));
}

#[test]
fn test_split_token_strips_punctuation() {
    assert_eq!(split_token("hello"), ("", "hello", ""));
    assert_eq!(split_token("(word)"), ("(", "word", ")"));
    assert_eq!(split_token("word,"), ("", "word", ","));
    assert_eq!(split_token("\"Hello\""), ("\"", "Hello", "\""));
    assert_eq!(split_token("123"), ("", "123", ""));
    assert_eq!(split_token(""), ("", "", ""));
}

#[test]
fn test_enhance_preserves_all_caps_style() {
    let out = StochasticEnhancer::new(1.0).enhance("CHAOS");
    assert_eq!(
        out.text,
        out.text.to_uppercase(),
        "ALL_CAPS input must yield ALL_CAPS output"
    );
}

#[test]
fn test_with_default_probability_is_half() {
    assert_eq!(
        StochasticEnhancer::with_default_probability().probability(),
        0.5
    );
}

#[test]
fn test_enhance_realistic_sentence_structure() {
    let input = "The chaos governs the universe and energy drives the pattern.";
    let out = StochasticEnhancer::new(0.8).enhance(input);
    assert!(
        out.text.to_lowercase().contains("the"),
        "stop word 'the' must remain in the output"
    );
    assert!(
        out.text.to_lowercase().contains("and"),
        "stop word 'and' must remain in the output"
    );
}
