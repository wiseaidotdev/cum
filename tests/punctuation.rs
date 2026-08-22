// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use cum_rs::unicode::{CleanOpts, DASH_HOMOGLYPHS, clean_text};

#[test]
fn test_normalize_punctuation_curly_quotes() {
    let mut opts = CleanOpts::safe();
    opts.normalize_punctuation = true;

    let input = "\u{201C}hello\u{201D} and \u{2018}world\u{2019}";
    let (cleaned, stats) = clean_text(input, &opts).unwrap();

    assert_eq!(
        cleaned, "\"hello\" and 'world'",
        "Curly quotes must be converted to ASCII equivalents"
    );
    assert!(stats.replaced_count >= 2);
}

#[test]
fn test_normalize_punctuation_ellipsis() {
    let mut opts = CleanOpts::safe();
    opts.normalize_punctuation = true;

    let input = "Wait\u{2026} what?";
    let (cleaned, stats) = clean_text(input, &opts).unwrap();

    assert_eq!(
        cleaned, "Wait... what?",
        "U+2026 horizontal ellipsis must expand to '...'"
    );
    assert_eq!(stats.replaced_count, 1);
}

#[test]
fn test_normalize_punctuation_off_preserves_chars() {
    let mut opts = CleanOpts::safe();
    opts.normalize_punctuation = false;

    let input = "\u{201C}hello\u{201D} wait\u{2026}";
    let (cleaned, _) = clean_text(input, &opts).unwrap();

    assert_eq!(
        cleaned, input,
        "Typography must be preserved when normalize_punctuation is false"
    );
}

#[test]
fn test_dash_homoglyphs_replaced_automatically() {
    let opts = CleanOpts::safe();

    for &(cp, expected) in DASH_HOMOGLYPHS {
        if let Some(ch) = char::from_u32(cp) {
            let input = format!("A{ch}B");
            let (cleaned, stats) = clean_text(&input, &opts).unwrap();
            let expected_str = format!("A{expected}B");

            assert_eq!(cleaned, expected_str, "U+{cp:04X} must map to '{expected}'");
            assert_eq!(stats.replaced_count, 1, "U+{cp:04X} must count as replaced");
        }
    }
}
