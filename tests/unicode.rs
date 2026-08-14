// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unit tests for the Layer-A Unicode watermark detection and removal engine.

use cum_rs::unicode::{
    CleanOpts, InspectOpts, SPACE_HOMOGLYPHS, STRIP_CODEPOINTS, clean_text, inspect_text,
};

fn default_clean_opts() -> CleanOpts {
    CleanOpts::safe()
}

fn default_inspect_opts() -> InspectOpts {
    InspectOpts::default()
}

#[test]
fn test_clean_empty_string() {
    let (cleaned, stats) = clean_text("", &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "");
    assert_eq!(stats.removed_count, 0);
    assert_eq!(stats.replaced_count, 0);
}

#[test]
fn test_clean_plain_ascii() {
    let input = "Hello, world!";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, input);
    assert_eq!(stats.removed_count, 0);
}

#[test]
fn test_clean_only_watermarks_gives_empty() {
    let input = "\u{200B}\u{FEFF}\u{2060}";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "");
    assert_eq!(stats.removed_count, 3);
}

#[test]
fn test_every_strip_codepoint_is_removed() {
    for &cp in STRIP_CODEPOINTS {
        if let Some(ch) = char::from_u32(cp) {
            let input = format!("A{ch}B");
            let (cleaned, stats) = clean_text(&input, &default_clean_opts()).unwrap();
            assert_eq!(cleaned, "AB", "codepoint U+{cp:04X} was not stripped");
            assert!(stats.removed_count >= 1, "codepoint U+{cp:04X} not counted");
        }
    }
}

#[test]
fn test_zwsp_removal() {
    let input = "Hello\u{200B} world!";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "Hello world!");
    assert_eq!(stats.removed_count, 1);
}

#[test]
fn test_bom_removal() {
    let input = "text\u{FEFF}content";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "textcontent");
    assert_eq!(stats.removed_count, 1);
}

#[test]
fn test_word_joiner_removal() {
    let input = "word\u{2060}joiner";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "wordjoiner");
    assert_eq!(stats.removed_count, 1);
}

#[test]
fn test_bidi_controls_removed() {
    let bidi_chars = ['\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}'];
    for ch in bidi_chars {
        let input = format!("A{ch}B");
        let (cleaned, _stats) = clean_text(&input, &default_clean_opts()).unwrap();
        assert_eq!(cleaned, "AB", "bidi char U+{:04X} not stripped", ch as u32);
    }
}

#[test]
fn test_tag_chars_removed() {
    let tag_chars = ['\u{E0001}', '\u{E007F}', '\u{E0041}'];
    for ch in tag_chars {
        let input = format!("A{ch}B");
        let (cleaned, _stats) = clean_text(&input, &default_clean_opts()).unwrap();
        assert_eq!(cleaned, "AB", "tag char U+{:04X} not stripped", ch as u32);
    }
}

#[test]
fn test_private_use_removed() {
    let pua_chars = ['\u{E001}', '\u{F8FF}', '\u{F0001}'];
    for ch in pua_chars {
        let input = format!("A{ch}B");
        let (cleaned, _stats) = clean_text(&input, &default_clean_opts()).unwrap();
        assert_eq!(cleaned, "AB", "PUA char U+{:04X} not stripped", ch as u32);
    }
}

#[test]
fn test_space_homoglyphs_normalized() {
    let opts = CleanOpts::safe();
    for &(cp, _) in SPACE_HOMOGLYPHS {
        if let Some(ch) = char::from_u32(cp) {
            let input = format!("A{ch}B");
            let (cleaned, stats) = clean_text(&input, &opts).unwrap();
            assert_eq!(cleaned, "A B", "space homoglyph U+{cp:04X} not replaced");
            assert_eq!(stats.replaced_count, 1);
        }
    }
}

#[test]
fn test_latin_confusables_aggressive_mode() {
    let opts = CleanOpts {
        aggressive_confusables: true,
        normalize_spaces: true,
        nfkc: false,
        strip_emoji_glue: false,
    };
    let cyrillic_a = '\u{0410}';
    let input = format!("{cyrillic_a}pple");
    let (cleaned, stats) = clean_text(&input, &opts).unwrap();
    assert_eq!(cleaned, "Apple");
    assert_eq!(stats.replaced_count, 1);
}

#[test]
fn test_confusables_not_replaced_in_normal_mode() {
    let opts = CleanOpts {
        aggressive_confusables: false,
        normalize_spaces: true,
        nfkc: false,
        strip_emoji_glue: false,
    };
    let cyrillic_a = '\u{0410}';
    let input = format!("{cyrillic_a}pple");
    let (cleaned, _stats) = clean_text(&input, &opts).unwrap();
    assert_eq!(
        cleaned, input,
        "Cyrillic should not be replaced in normal mode"
    );
}

#[test]
fn test_emoji_glue_preserved_after_emoji_base() {
    let input = "❤️‍🔥";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, input, "emoji ZWJ sequence should be preserved");
    assert_eq!(stats.removed_count, 0);
}

#[test]
fn test_zwj_stripped_when_isolated() {
    let input = "\u{200D}isolated";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "isolated");
    assert_eq!(stats.removed_count, 1);
}

#[test]
fn test_flag_emoji_tag_chars_preserved() {
    let flag_gb = "🏴󠁧󠁢󠁥󠁮󠁧󠁿";
    let (cleaned, stats) = clean_text(flag_gb, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, flag_gb, "flag emoji tag chars should be preserved");
    assert_eq!(stats.removed_count, 0);
}

#[test]
fn test_inspect_finds_zwsp() {
    let report = inspect_text("A\u{200B}B", &default_inspect_opts()).unwrap();
    assert_eq!(report.suspicious_total, 1);
    assert_eq!(report.hits.len(), 1);
    assert_eq!(report.hits[0].codepoint, 0x200B);
}

#[test]
fn test_inspect_multiple_codepoints() {
    let report = inspect_text("A\u{200B}B\u{FEFF}C\u{200B}D", &default_inspect_opts()).unwrap();
    assert_eq!(report.suspicious_total, 3);
    let zwsp_hit = report.hits.iter().find(|h| h.codepoint == 0x200B).unwrap();
    assert_eq!(zwsp_hit.count, 2);
}

#[test]
fn test_inspect_empty_gives_no_hits() {
    let report = inspect_text("", &default_inspect_opts()).unwrap();
    assert_eq!(report.suspicious_total, 0);
    assert!(report.hits.is_empty());
}

#[test]
fn test_inspect_reports_length() {
    let text = "Hello!";
    let report = inspect_text(text, &default_inspect_opts()).unwrap();
    assert_eq!(report.length, text.chars().count());
}

#[test]
fn test_inspect_hits_sorted_by_count_descending() {
    let report = inspect_text("\u{200B}\u{200B}\u{FEFF}", &default_inspect_opts()).unwrap();
    if report.hits.len() >= 2 {
        assert!(report.hits[0].count >= report.hits[1].count);
    }
}

#[test]
fn test_clean_text_multiple_zwsp() {
    let input = "a\u{200B}b\u{200B}c\u{200B}d";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "abcd");
    assert_eq!(stats.removed_count, 3);
}

#[test]
fn test_variation_selectors_stripped() {
    let input = "A\u{FE00}B\u{FE0F}C";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "ABC");
    assert_eq!(stats.removed_count, 2);
}

#[test]
fn test_vs_supplement_stripped() {
    let vs17 = '\u{E0100}';
    let input = format!("A{vs17}B");
    let (cleaned, stats) = clean_text(&input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "AB");
    assert_eq!(stats.removed_count, 1);
}

#[test]
fn test_unicode_with_mixed_content() {
    let input = "Claude\u{200B}is\u{FEFF}watching\u{2060}you";
    let (cleaned, stats) = clean_text(input, &default_clean_opts()).unwrap();
    assert_eq!(cleaned, "Claudeiswatchingyou");
    assert_eq!(stats.removed_count, 3);
}

#[test]
fn test_large_text_performance() {
    let base = "Hello world! ";
    let large: String = base.repeat(10_000);
    let (cleaned, stats) = clean_text(&large, &default_clean_opts()).unwrap();
    assert_eq!(cleaned.len(), large.len());
    assert_eq!(stats.removed_count, 0);
}
