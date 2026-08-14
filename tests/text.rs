// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for Layer-A text watermark round-trips.

use cum_rs::cleaner::clean;
use cum_rs::types::MediaHint;
use cum_rs::unicode::{CleanOpts, InspectOpts, STRIP_CODEPOINTS, clean_text, inspect_text};

/// Verify every codepoint in STRIP_CODEPOINTS is absent after cleaning.
#[test]
fn test_all_strip_codepoints_absent_after_clean() {
    let watermarks: String = STRIP_CODEPOINTS
        .iter()
        .filter_map(|&cp| char::from_u32(cp))
        .collect();

    let input = format!("Hello {watermarks} world");
    let (cleaned, stats) = clean_text(&input, &CleanOpts::safe()).unwrap();

    for &cp in STRIP_CODEPOINTS {
        if let Some(ch) = char::from_u32(cp) {
            assert!(
                !cleaned.contains(ch),
                "codepoint U+{cp:04X} was not removed from cleaned output"
            );
        }
    }

    assert!(stats.removed_count > 0);
    assert!(cleaned.contains("Hello"));
    assert!(cleaned.contains("world"));
}

/// Verify inspect → clean is consistent: every hit found by inspect is absent
/// in the cleaned output.
#[test]
fn test_inspect_then_clean_consistent() {
    let inputs = [
        "fine text",
        "A\u{200B}B",
        "C\u{FEFF}D\u{200E}E",
        "\u{202A}force-ltr\u{202C}",
        "\u{E0041}tag-char\u{E007F}",
    ];

    for input in &inputs {
        let report = inspect_text(input, &InspectOpts::default()).unwrap();
        let (cleaned, stats) = clean_text(input, &CleanOpts::safe()).unwrap();

        assert_eq!(
            report.suspicious_total,
            stats.removed_count + stats.replaced_count,
            "inspect count {}, clean count {} mismatch for {:?}",
            report.suspicious_total,
            stats.removed_count + stats.replaced_count,
            input
        );

        for hit in &report.hits {
            if let Some(ch) = char::from_u32(hit.codepoint) {
                assert!(
                    !cleaned.contains(ch),
                    "char U+{:04X} found by inspect but still in cleaned output for {:?}",
                    hit.codepoint,
                    input
                );
            }
        }
    }
}

/// Clean twice and verify idempotence.
#[test]
fn test_clean_text_is_idempotent() {
    let input = "Hello\u{200B} world\u{FEFF}! Text with \u{2060}marks.";
    let opts = CleanOpts::safe();
    let (pass1, _) = clean_text(input, &opts).unwrap();
    let (pass2, stats2) = clean_text(&pass1, &opts).unwrap();
    assert_eq!(pass1, pass2, "second pass should not change the output");
    assert_eq!(stats2.removed_count, 0);
    assert_eq!(stats2.replaced_count, 0);
}

/// Verify the unified clean() API on text bytes matches clean_text().
#[test]
fn test_unified_clean_text_matches_clean_text_fn() {
    let input = "Hello\u{200B} world\u{FEFF}!";
    let direct = {
        let (s, _) = clean_text(input, &CleanOpts::safe()).unwrap();
        s
    };
    let via_api = {
        let out = clean(input.as_bytes(), Some(MediaHint::Text)).unwrap();
        String::from_utf8(out.bytes).unwrap()
    };
    assert_eq!(direct, via_api);
}

/// Verify that Unicode surrounding non-ASCII letters is handled correctly.
#[test]
fn test_script_joiner_safe_in_arabic_word() {
    let arabic = "عربي";
    let (cleaned, _stats) = clean_text(arabic, &CleanOpts::safe()).unwrap();
    assert_eq!(cleaned, arabic, "Arabic word should be preserved unchanged");
}

/// Verify Devanagari half-form conjunct preserved.
#[test]
fn test_devanagari_virama_preserved() {
    let devanagari = "क्ष";
    let (cleaned, _stats) = clean_text(devanagari, &CleanOpts::safe()).unwrap();
    assert_eq!(
        cleaned, devanagari,
        "Devanagari conjunct should be preserved"
    );
}

/// End-to-end: varied watermark carriers in a realistic sentence.
#[test]
fn test_realistic_watermarked_sentence() {
    let text = "The\u{200B} quick\u{FEFF} brown\u{2060} fox\u{200E} jumps\u{202A} over\u{202C} the lazy dog.";
    let (cleaned, stats) = clean_text(text, &CleanOpts::safe()).unwrap();
    assert_eq!(cleaned, "The quick brown fox jumps over the lazy dog.");
    assert_eq!(stats.removed_count, 6);
    assert_eq!(stats.replaced_count, 0);
}
