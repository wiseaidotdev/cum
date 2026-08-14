// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unit tests for CumError variants and propagation.

use cum_rs::cleaner::clean;
use cum_rs::error::CumError;
use cum_rs::types::MediaHint;
use cum_rs::unicode::clean_text;

#[test]
fn test_input_too_large_from_clean() {
    use cum_rs::cleaner::MAX_INPUT_BYTES;
    let data = vec![b'x'; MAX_INPUT_BYTES + 1];
    let err = clean(&data, Some(MediaHint::Text)).unwrap_err();
    assert!(matches!(err, CumError::InputTooLarge { .. }));
    assert!(err.to_string().contains("too large"));
}

#[test]
fn test_unsupported_format_from_clean() {
    let garbage = b"\x01\x02\x03\x04\x05\x06\x07\x08\xFF\xFE";
    let err = clean(garbage, None).unwrap_err();
    assert!(matches!(err, CumError::UnsupportedFormat(_)));
}

#[test]
fn test_parse_error_from_clean_jpeg_truncated() {
    let truncated_jpeg = b"\xFF\xD8\xFF\xE1\x00\x40"; // truncated APP1
    let err = clean(truncated_jpeg, Some(MediaHint::Jpeg)).unwrap_err();
    assert!(matches!(err, CumError::ParseError(_)));
}

#[test]
fn test_input_too_large_from_clean_text() {
    use cum_rs::unicode::{CleanOpts, MAX_TEXT_CHARS};
    let s: String = "a".repeat(MAX_TEXT_CHARS + 1);
    let err = clean_text(&s, &CleanOpts::safe()).unwrap_err();
    assert!(matches!(err, CumError::InputTooLarge { .. }));
}

#[test]
fn test_cum_error_display_messages() {
    let e = CumError::UnsupportedFormat("test".into());
    assert!(e.to_string().contains("unsupported format"));

    let e2 = CumError::ParseError("bad data".into());
    assert!(e2.to_string().contains("parse error"));

    let e3 = CumError::InputTooLarge {
        limit: 100,
        actual: 200,
    };
    assert!(e3.to_string().contains("100"));
    assert!(e3.to_string().contains("200"));

    let e4 = CumError::BinaryInput("PNG image".into());
    assert!(e4.to_string().contains("binary"));

    let e5 = CumError::Zip("zip failed".into());
    assert!(e5.to_string().contains("zip"));
}

#[test]
fn test_io_error_wrapping() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let cum_err: CumError = io_err.into();
    assert!(matches!(cum_err, CumError::Io(_)));
    assert!(cum_err.to_string().contains("I/O error"));
}
