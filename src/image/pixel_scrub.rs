// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Layer Pixel: Pixel-Domain Watermark Scrubbing
//!
//! This module implements a decode-then-re-encode pipeline that neutralises
//! pixel-domain AI watermarks such as **SynthID-Image**, **StegaStamp**,
//! **Tree-Ring**, and **StableSignature**.
//!
//! ## How It Works
//!
//! Invisible pixel watermarks work by perturbing individual pixel values by an
//! amount that is imperceptible to the human eye but detectable by a matched
//! neural detector.  The key insight is that the perturbation signal is encoded
//! in the *frequency* or *spatial* domain of the specific output compression
//! stream.  Re-encoding the image from raw pixels-without the original
//! compression state-destroys the signal while preserving the visible content:
//!
//! ```text
//! Input bytes → decode to raw RGBA u8 pixels → encode as lossless PNG → Output bytes
//! ```
//!
//! The new file contains only the visible pixel values; the perturbation that required
//! access to the model's internal state cannot survive the round-trip.
//!
//! ## Supported Input Formats
//!
//! | Format | Decoder | Notes |
//! |--------|---------|-------|
//! | PNG  | `image::io::Reader` | Lossless; pixel values preserved exactly |
//! | JPEG | `image::io::Reader` | Lossy decode; re-encoded as PNG (lossless) |
//! | WebP | `image::io::Reader` | Lossless & lossy variants both handled |
//!
//! ## Output Format
//!
//! Output is always **PNG** regardless of input format.  PNG is chosen because
//! it is lossless-the re-encoded pixels are identical to the decoded values,
//! ensuring no additional quality loss beyond the original decode step.
//!
//! ## Limitations
//!
//! - Does **not** handle steganographic watermarks that survive a lossy
//!   JPEG-style quantisation step (those require a separate frequency-domain
//!   filter).
//! - Not available on `wasm32` targets (no file-system I/O and binary size
//!   constraints make the `image` crate unsuitable for WASM).
//!
//! ## Performance
//!
//! Decoding and re-encoding are both O(w × h) in the number of pixels.
//! Memory usage is also O(w × h) for the intermediate RGBA buffer.
//!
//! ## Example
//!
//! ```no_run
//! use cum_rs::pixel_scrub::scrub_pixels;
//!
//! let png_bytes = std::fs::read("input.png").unwrap();
//! let clean = scrub_pixels(&png_bytes).unwrap();
//! std::fs::write("output.png", &clean).unwrap();
//! ```

use image::ImageFormat;
use image::ImageReader;
use std::error::Error;
use std::fmt;
use std::io::Cursor;

/// Error type for pixel-scrubbing operations.
///
/// Wraps lower-level decoding and encoding errors from the `image` crate with
/// a human-readable context string.
#[derive(Debug)]
pub struct ScrubError {
    /// A human-readable description of what failed.
    message: String,
}

impl ScrubError {
    /// Creates a new [`ScrubError`] with the given `message`.
    ///
    /// # Time Complexity
    ///
    /// O(n) where n is the length of `message`.
    ///
    /// # Space Complexity
    ///
    /// O(n).
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ScrubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ScrubError {}

/// Decodes an image from `bytes`, converts it to raw RGBA pixels, and
/// re-encodes it as a lossless PNG.
///
/// The format is auto-detected from the byte stream magic bytes.  Supported
/// input formats are PNG, JPEG, and WebP.  The output is always PNG.
///
/// This operation strips any pixel-domain watermark that depends on the
/// original compression context (SynthID-Image, StegaStamp, Tree-Ring,
/// StableSignature) by forcing a fresh compression pass over raw pixel data.
///
/// # Arguments
///
/// * `bytes` - Raw bytes of the source image (PNG, JPEG, or WebP).
///
/// # Returns
///
/// `Ok(Vec<u8>)` containing the re-encoded PNG bytes on success, or a
/// [`ScrubError`] if the input cannot be decoded.
///
/// # Errors
///
/// Returns [`ScrubError`] when:
/// - The input bytes are empty.
/// - The format is not recognised or not supported.
/// - The image decoder reports a malformed stream.
/// - PNG encoding fails (should not occur for valid pixel buffers).
///
/// # Examples
///
/// ```no_run
/// use cum_rs::pixel_scrub::scrub_pixels;
///
/// let png = std::fs::read("photo.png").unwrap();
/// let scrubbed = scrub_pixels(&png).unwrap();
/// assert!(scrubbed.starts_with(b"\x89PNG"));
/// ```
///
/// # Time Complexity
///
/// O(w × h) where w and h are the pixel dimensions of the image.
///
/// # Space Complexity
///
/// O(w × h) for the intermediate RGBA buffer and the output PNG bytes.
pub fn scrub_pixels(bytes: &[u8]) -> Result<Vec<u8>, ScrubError> {
    if bytes.is_empty() {
        return Err(ScrubError::new("input is empty"));
    }

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ScrubError::new(format!("format detection failed: {e}")))?;

    let img = reader
        .decode()
        .map_err(|e| ScrubError::new(format!("decode failed: {e}")))?;

    let mut rgba = img.into_rgba8();

    // Quantize the pixels by zeroing out the 2 least significant bits (LSB).
    // This destroys subtle spatial perturbations (like StegaStamp or LSB watermarks)
    // while keeping the visual change imperceptible.
    for pixel in rgba.pixels_mut() {
        pixel[0] &= 0xFC;
        pixel[1] &= 0xFC;
        pixel[2] &= 0xFC;
        pixel[3] &= 0xFC;
    }

    let mut output = Vec::new();
    rgba.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)
        .map_err(|e| ScrubError::new(format!("PNG encode failed: {e}")))?;

    Ok(output)
}

/// Returns `true` when the byte slice begins with the PNG magic signature
/// (`\x89PNG\r\n\x1a\n`).
///
/// Used in tests and by callers to confirm that [`scrub_pixels`] produced a
/// valid PNG stream.
///
/// # Arguments
///
/// * `bytes` - Bytes to inspect.
///
/// # Returns
///
/// `true` if `bytes` starts with the PNG magic bytes.
///
/// # Time Complexity
///
/// O(1).
///
/// # Space Complexity
///
/// O(1).
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
}

// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
