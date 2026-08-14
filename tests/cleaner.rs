// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unit tests for the unified cleaner API.

use cum_rs::cleaner::{
    all_findings, clean, format_name, inspect, inspect_and_clean, resolve_format,
};
use cum_rs::types::MediaHint;

#[test]
fn test_clean_text_with_hint() {
    let dirty = "Hello\u{200B} world!";
    let output = clean(dirty.as_bytes(), Some(MediaHint::Text)).unwrap();
    let cleaned = String::from_utf8(output.bytes).unwrap();
    assert_eq!(cleaned, "Hello world!");
    assert_eq!(output.stats.removed_count, 1);
}

#[test]
fn test_clean_text_auto_detect() {
    let dirty = "Clean text with no watermarks";
    let output = clean(dirty.as_bytes(), None).unwrap();
    assert_eq!(output.bytes, dirty.as_bytes());
}

#[test]
fn test_inspect_text_with_hint() {
    let dirty = "A\u{200B}B";
    let output = inspect(dirty.as_bytes(), Some(MediaHint::Text)).unwrap();
    let report = output.text_report.unwrap();
    assert_eq!(report.suspicious_total, 1);
}

#[test]
fn test_inspect_text_auto_detect() {
    let dirty = "A\u{200B}B\u{FEFF}C";
    let output = inspect(dirty.as_bytes(), None).unwrap();
    assert!(output.text_report.is_some());
    assert_eq!(output.text_report.unwrap().suspicious_total, 2);
}

#[test]
fn test_clean_png_with_hint() {
    let png_magic = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0DIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90\x77\x53\xDE\x00\x00\x00\x00IEND\xAE\x42\x60\x82";
    let output = clean(png_magic, Some(MediaHint::Png));
    assert!(output.is_ok());
}

#[test]
fn test_clean_pdf_with_hint() {
    let pdf = b"%PDF-1.7\nsome content";
    let output = clean(pdf, Some(MediaHint::Pdf)).unwrap();
    assert!(!output.bytes.is_empty());
}

#[test]
fn test_clean_html_with_hint() {
    let html = b"<!DOCTYPE html><html><head><meta name='generator' content='TestAI'/></head><body>hello</body></html>";
    let output = clean(html, Some(MediaHint::Html)).unwrap();
    let cleaned = std::str::from_utf8(&output.bytes).unwrap();
    assert!(
        cleaned.contains("hello"),
        "body content should be preserved"
    );
}

#[test]
fn test_clean_markdown_with_hint() {
    let md = b"---\ngenerator: claude\n---\n# Hello\n";
    let output = clean(md, Some(MediaHint::Markdown)).unwrap();
    let cleaned = std::str::from_utf8(&output.bytes).unwrap();
    assert!(!cleaned.contains("generator: claude"));
    assert!(cleaned.contains("# Hello"));
}

#[test]
fn test_inspect_and_clean_returns_both() {
    let dirty = "A\u{200B}B";
    let (inspect_out, clean_out) = inspect_and_clean(dirty.as_bytes(), None).unwrap();
    assert!(inspect_out.text_report.is_some());
    assert_eq!(String::from_utf8(clean_out.bytes).unwrap(), "AB");
}

#[test]
fn test_unsupported_format_errors() {
    let garbage = b"\x00\x01\x02\x03\x04\x05\xFF\xFE\xFD";
    let result = clean(garbage, None);
    assert!(result.is_err());
}

#[test]
fn test_input_too_large_errors() {
    use cum_rs::cleaner::MAX_INPUT_BYTES;
    let large = vec![b'A'; MAX_INPUT_BYTES + 1];
    let result = clean(&large, Some(MediaHint::Text));
    assert!(result.is_err());
}

#[test]
fn test_resolve_format_png() {
    let png = b"\x89PNG\r\n\x1a\n";
    assert_eq!(resolve_format(png, None), Some(MediaHint::Png));
}

#[test]
fn test_resolve_format_jpeg() {
    let jpeg = b"\xFF\xD8\xFF";
    assert_eq!(resolve_format(jpeg, None), Some(MediaHint::Jpeg));
}

#[test]
fn test_resolve_format_pdf() {
    let pdf = b"%PDF-1.7";
    assert_eq!(resolve_format(pdf, None), Some(MediaHint::Pdf));
}

#[test]
fn test_resolve_format_hint_overrides() {
    let text = b"Hello world";
    assert_eq!(
        resolve_format(text, Some(&MediaHint::Markdown)),
        Some(MediaHint::Markdown)
    );
}

#[test]
fn test_format_name_values() {
    assert_eq!(format_name(&MediaHint::Text), "plain text");
    assert_eq!(format_name(&MediaHint::Png), "PNG image");
    assert_eq!(format_name(&MediaHint::Pdf), "PDF document");
}

#[test]
fn test_all_findings_empty_when_no_reports() {
    let output = cum_rs::types::InspectOutput {
        text_report: None,
        image_report: None,
        meta_findings: vec![],
        format: MediaHint::Text,
    };
    assert!(all_findings(&output).is_empty());
}

#[test]
fn test_all_findings_collects_meta_findings() {
    let finding = cum_rs::types::MetaFinding {
        description: "test finding".into(),
        confidence: cum_rs::types::Confidence::Probable,
        kind: Some(cum_rs::types::WatermarkKind::C2paMetadata),
    };
    let output = cum_rs::types::InspectOutput {
        text_report: None,
        image_report: None,
        meta_findings: vec![finding],
        format: MediaHint::Pdf,
    };
    assert_eq!(all_findings(&output).len(), 1);
}
