// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(feature = "pixel-scrub")]

use cum_rs::image::pixel_scrub::{is_png, scrub_pixels};
use image::{ImageFormat, RgbImage};
use std::io::Cursor;

fn create_test_image(format: ImageFormat) -> Vec<u8> {
    let img = RgbImage::new(10, 10);
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), format).unwrap();
    bytes
}

#[test]
fn test_scrub_empty_input() {
    let err = scrub_pixels(&[]).unwrap_err();
    assert_eq!(err.to_string(), "input is empty");
}

#[test]
fn test_scrub_invalid_input() {
    let invalid = b"this is not an image";
    let err = scrub_pixels(invalid).unwrap_err();
    assert!(
        err.to_string().contains("format detection failed")
            || err.to_string().contains("decode failed")
    );
}

#[test]
fn test_is_png_magic_bytes() {
    let png_magic = b"\x89PNG\r\n\x1a\nrest of file";
    assert!(is_png(png_magic));

    let not_png = b"\xff\xd8\xff\xe0";
    assert!(!is_png(not_png));
}

#[test]
fn test_scrub_png_returns_png_and_preserves_dimensions() {
    let png_bytes = create_test_image(ImageFormat::Png);
    assert!(is_png(&png_bytes));

    let scrubbed = scrub_pixels(&png_bytes).expect("should scrub valid PNG");
    assert!(is_png(&scrubbed));
    assert!(!scrubbed.is_empty());

    let decoded = image::load_from_memory(&scrubbed).unwrap();
    assert_eq!(decoded.width(), 10);
    assert_eq!(decoded.height(), 10);
}

#[test]
fn test_scrub_jpeg_returns_png_and_preserves_dimensions() {
    let jpeg_bytes = create_test_image(ImageFormat::Jpeg);
    assert!(!is_png(&jpeg_bytes));

    let scrubbed = scrub_pixels(&jpeg_bytes).expect("should scrub valid JPEG");
    assert!(is_png(&scrubbed));
    assert!(!scrubbed.is_empty());

    let decoded = image::load_from_memory(&scrubbed).unwrap();
    assert_eq!(decoded.width(), 10);
    assert_eq!(decoded.height(), 10);
}

#[test]
fn test_scrub_removes_lsb_watermark() {
    let mut img = RgbImage::new(10, 10);
    for pixel in img.pixels_mut() {
        // base color 100, plus watermark bits 3 (which is 0b11)
        pixel[0] = 100 | 0b11;
        pixel[1] = 100 | 0b10;
        pixel[2] = 100 | 0b01;
    }
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();

    let scrubbed = scrub_pixels(&bytes).expect("should scrub successfully");

    let decoded = image::load_from_memory(&scrubbed).unwrap().into_rgb8();
    for pixel in decoded.pixels() {
        assert_eq!(pixel[0] & 0b11, 0, "Red channel LSB should be 0");
        assert_eq!(pixel[1] & 0b11, 0, "Green channel LSB should be 0");
        assert_eq!(pixel[2] & 0b11, 0, "Blue channel LSB should be 0");

        assert_eq!(pixel[0], 100);
        assert_eq!(pixel[1], 100);
        assert_eq!(pixel[2], 100);
    }
}
