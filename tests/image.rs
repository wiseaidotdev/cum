// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for image metadata cleaning round-trips.

use cum_rs::cleaner::clean;
use cum_rs::image_meta::{clean_image, inspect_image};
use cum_rs::types::{Confidence, MediaHint};

/// Builds a minimal PNG with a C2PA chunk injected before IEND.
fn make_png_with_c2pa() -> Vec<u8> {
    let crc32 = |data: &[u8]| -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    };

    let mut png = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE,
    ];

    let c2pa_data = b"fake-c2pa-manifest";
    let crc = crc32(&[b"C2PA", c2pa_data.as_ref()].concat());
    png.extend_from_slice(&(c2pa_data.len() as u32).to_be_bytes());
    png.extend_from_slice(b"C2PA");
    png.extend_from_slice(c2pa_data);
    png.extend_from_slice(&crc.to_be_bytes());

    png.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]);
    png
}

#[test]
fn test_png_c2pa_round_trip() {
    let dirty = make_png_with_c2pa();

    let report = inspect_image(&dirty).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.description.contains("C2PA")),
        "inspect should find C2PA chunk before cleaning"
    );

    let cleaned = clean_image(&dirty).unwrap();

    assert!(
        cleaned.windows(4).all(|w| w != b"C2PA"),
        "C2PA chunk bytes should be absent from cleaned PNG"
    );

    assert!(
        cleaned.starts_with(b"\x89PNG\r\n\x1a\n"),
        "cleaned PNG must start with magic"
    );

    let report_after = inspect_image(&cleaned).unwrap();
    assert!(
        !report_after
            .findings
            .iter()
            .any(|f| f.description.contains("C2PA")),
        "inspect after cleaning should find no C2PA chunk"
    );
}

#[test]
fn test_png_clean_via_unified_api() {
    let dirty = make_png_with_c2pa();
    let output = clean(&dirty, Some(MediaHint::Png)).unwrap();

    assert!(
        output.bytes.windows(4).all(|w| w != b"C2PA"),
        "unified clean should strip C2PA"
    );
    assert_eq!(output.format, MediaHint::Png);
}

#[test]
fn test_png_clean_is_still_valid_png() {
    let dirty = make_png_with_c2pa();
    let cleaned = clean_image(&dirty).unwrap();

    assert!(cleaned.starts_with(b"\x89PNG\r\n\x1a\n"), "magic preserved");
    assert!(
        cleaned.windows(4).any(|w| w == b"IHDR"),
        "IHDR chunk preserved"
    );
    assert!(
        cleaned.windows(4).any(|w| w == b"IEND"),
        "IEND chunk preserved"
    );
}

#[test]
fn test_png_c2pa_confidence_is_confirmed() {
    let dirty = make_png_with_c2pa();
    let report = inspect_image(&dirty).unwrap();
    let c2pa_finding = report
        .findings
        .iter()
        .find(|f| f.description.contains("C2PA"));
    assert!(c2pa_finding.is_some());
    assert_eq!(c2pa_finding.unwrap().confidence, Confidence::Confirmed);
}

#[test]
fn test_jpeg_exif_round_trip() {
    let mut dirty = vec![0xFF, 0xD8]; // SOI
    let exif = b"Exif\0\0fake-exif-by-claude";
    let seg_len = (exif.len() as u16 + 2).to_be_bytes();
    dirty.push(0xFF);
    dirty.push(0xE1); // APP1
    dirty.extend_from_slice(&seg_len);
    dirty.extend_from_slice(exif);
    dirty.extend_from_slice(&[0xFF, 0xD9]); // EOI

    let report = inspect_image(&dirty).unwrap();
    assert!(
        !report.findings.is_empty(),
        "should find EXIF before cleaning"
    );

    let cleaned = clean_image(&dirty).unwrap();

    assert!(
        !cleaned.windows(2).any(|w| w == [0xFF, 0xE1]),
        "APP1 (EXIF) segment should be absent from cleaned JPEG"
    );
    assert!(cleaned.starts_with(&[0xFF, 0xD8]), "SOI preserved");
    assert!(cleaned.ends_with(&[0xFF, 0xD9]), "EOI preserved");
}

#[test]
fn test_svg_metadata_round_trip() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><metadata><ai:model>claude-3.5</ai:model></metadata><circle r=\"5\" fill=\"red\"/></svg>";

    let report = inspect_image(svg).unwrap();
    assert!(
        !report.findings.is_empty(),
        "inspect should detect SVG metadata"
    );

    let cleaned = clean_image(svg).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();
    assert!(
        !cleaned_str.contains("<metadata>"),
        "metadata block removed"
    );
    assert!(cleaned_str.contains("<circle"), "SVG content preserved");
}
