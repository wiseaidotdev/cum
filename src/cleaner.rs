// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Unified Watermark Cleaner
//!
//! High-level API that dispatches to the appropriate cleaning backend based on
//! the media format, auto-detecting it from magic bytes when no hint is given.
//!
//! ## Entry Points
//!
//! | Function | Description |
//! |----------|-------------|
//! | [`clean`] | Remove all detectable watermarks; return cleaned bytes + stats. |
//! | [`inspect`] | Enumerate all detectable watermarks without modifying input. |
//!
//! ## Auto-detection Order
//!
//! 1. PNG magic (`\x89PNG`)
//! 2. JPEG magic (`\xFF\xD8\xFF`)
//! 3. WebP RIFF magic
//! 4. PDF magic (`%PDF-`)
//! 5. ZIP container (DOCX/ODT): `PK\x03\x04`
//! 6. HTML heuristic (`<!doctype html`, `<html`)
//! 7. SVG heuristic (`<svg`)
//! 8. UTF-8 text fallback (Layer-A Unicode clean)
//!
//! ## Example
//! ```
//! use cum_rs::cleaner::{clean, inspect};
//! use cum_rs::types::MediaHint;
//!
//! let watermarked_text = "Hello\u{200B} world\u{FEFF}!";
//! let output = clean(watermarked_text.as_bytes(), Some(MediaHint::Text)).unwrap();
//! assert_eq!(output.stats.removed_count, 2);
//! ```

use crate::container_meta::{ContainerFormat, clean_file, inspect_file};
use crate::error::{CumError, Result};
use crate::image_meta::{clean_image, detect_image_format, inspect_image};
use crate::types::{CleanOutput, CleanStats, ImageFormat, InspectOutput, MediaHint, MetaFinding};
use crate::unicode::{CleanOpts, InspectOpts, clean_text, inspect_text};

/// Maximum input size for the unified `clean` and `inspect` functions.
///
/// 256 MiB: processing whole files in memory makes unlimited input a
/// memory-exhaustion attack vector.
pub const MAX_INPUT_BYTES: usize = 256 * 1024 * 1024;

/// Resolves the [`MediaHint`] for a byte slice, using auto-detection when the
/// hint is `None`.
///
/// Returns `None` if the format cannot be determined.
///
/// # Complexity
/// - Time: O(1): only inspects the first few bytes.
/// - Space: O(1).
pub fn resolve_format(bytes: &[u8], hint: Option<&MediaHint>) -> Option<MediaHint> {
    if let Some(h) = hint {
        return Some(h.clone());
    }

    if let Some(img_fmt) = detect_image_format(bytes) {
        return Some(match img_fmt {
            ImageFormat::Png => MediaHint::Png,
            ImageFormat::Jpeg => MediaHint::Jpeg,
            ImageFormat::Webp => MediaHint::Webp,
            ImageFormat::Svg => MediaHint::Svg,
        });
    }

    if bytes.starts_with(b"%PDF-") {
        return Some(MediaHint::Pdf);
    }

    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        return Some(MediaHint::Docx);
    }

    if ContainerFormat::detect(bytes).is_some() {
        return Some(MediaHint::Html);
    }

    if std::str::from_utf8(bytes).is_ok() {
        return Some(MediaHint::Text);
    }

    None
}

/// Removes all detectable watermarks from the given byte slice.
///
/// # Arguments
/// * `bytes`: raw input bytes (text, image, or document).
/// * `hint`: optional media format hint; auto-detected when `None`.
///
/// # Returns
/// A [`CleanOutput`] containing cleaned bytes, statistics, and the resolved
/// media format.
///
/// # Errors
/// - [`CumError::InputTooLarge`] if `bytes.len() > MAX_INPUT_BYTES`.
/// - [`CumError::UnsupportedFormat`] if the format cannot be determined.
/// - [`CumError::ParseError`] if the byte stream is malformed.
///
/// # Complexity
/// - Time: O(n) for all formats except ZIP containers (O(n log n) due to
///   Deflate recompression).
/// - Space: O(n).
pub fn clean(bytes: &[u8], hint: Option<MediaHint>) -> Result<CleanOutput> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CumError::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: bytes.len(),
        });
    }

    let format = resolve_format(bytes, hint.as_ref()).ok_or_else(|| {
        CumError::UnsupportedFormat(
            "could not detect media format; pass an explicit MediaHint".into(),
        )
    })?;

    match &format {
        MediaHint::Text => {
            let text = std::str::from_utf8(bytes)
                .map_err(|e| CumError::ParseError(format!("not valid UTF-8: {e}")))?;
            let opts = CleanOpts::safe();
            let (cleaned, stats) = clean_text(text, &opts)?;
            Ok(CleanOutput {
                bytes: cleaned.into_bytes(),
                stats,
                format,
            })
        }

        MediaHint::Png | MediaHint::Jpeg | MediaHint::Webp | MediaHint::Svg => {
            let cleaned = clean_image(bytes)?;
            let removed = bytes.len().abs_diff(cleaned.len());
            Ok(CleanOutput {
                bytes: cleaned,
                stats: CleanStats {
                    removed_count: 0,
                    replaced_count: 0,
                    metadata_chunks_removed: usize::from(removed > 0),
                    summary: vec!["Image metadata stripped.".into()],
                },
                format,
            })
        }

        MediaHint::Pdf => {
            let (cleaned, stats) = clean_file(bytes, &ContainerFormat::Pdf)?;
            Ok(CleanOutput {
                bytes: cleaned,
                stats,
                format,
            })
        }

        MediaHint::Docx => {
            let (cleaned, stats) = clean_file(bytes, &ContainerFormat::Docx)?;
            Ok(CleanOutput {
                bytes: cleaned,
                stats,
                format,
            })
        }

        MediaHint::Odt => {
            let (cleaned, stats) = clean_file(bytes, &ContainerFormat::Odt)?;
            Ok(CleanOutput {
                bytes: cleaned,
                stats,
                format,
            })
        }

        MediaHint::Html => {
            let (cleaned, stats) = clean_file(bytes, &ContainerFormat::Html)?;
            Ok(CleanOutput {
                bytes: cleaned,
                stats,
                format,
            })
        }

        MediaHint::Markdown => {
            let (cleaned, stats) = clean_file(bytes, &ContainerFormat::Markdown)?;
            Ok(CleanOutput {
                bytes: cleaned,
                stats,
                format,
            })
        }
    }
}

/// Enumerates all detectable watermarks in the given byte slice without
/// modifying anything.
///
/// # Arguments
/// * `bytes`: raw input bytes.
/// * `hint`: optional media format hint; auto-detected when `None`.
///
/// # Returns
/// An [`InspectOutput`] describing all findings across all applicable detection
/// layers.
///
/// # Errors
/// - [`CumError::InputTooLarge`] if `bytes.len() > MAX_INPUT_BYTES`.
/// - [`CumError::UnsupportedFormat`] if the format cannot be determined.
///
/// # Complexity
/// - Time: O(n).
/// - Space: O(k) where k is the number of findings.
pub fn inspect(bytes: &[u8], hint: Option<MediaHint>) -> Result<InspectOutput> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CumError::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: bytes.len(),
        });
    }

    let format = resolve_format(bytes, hint.as_ref()).ok_or_else(|| {
        CumError::UnsupportedFormat(
            "could not detect media format; pass an explicit MediaHint".into(),
        )
    })?;

    match &format {
        MediaHint::Text => {
            let text = std::str::from_utf8(bytes)
                .map_err(|e| CumError::ParseError(format!("not valid UTF-8: {e}")))?;
            let opts = InspectOpts::default();
            let report = inspect_text(text, &opts)?;
            Ok(InspectOutput {
                text_report: Some(report),
                image_report: None,
                meta_findings: vec![],
                format,
            })
        }

        MediaHint::Png | MediaHint::Jpeg | MediaHint::Webp | MediaHint::Svg => {
            let report = inspect_image(bytes)?;
            Ok(InspectOutput {
                text_report: None,
                image_report: Some(report),
                meta_findings: vec![],
                format,
            })
        }

        MediaHint::Pdf => {
            let findings = inspect_file(bytes, &ContainerFormat::Pdf);
            Ok(InspectOutput {
                text_report: None,
                image_report: None,
                meta_findings: findings,
                format,
            })
        }

        MediaHint::Docx => {
            let findings = inspect_file(bytes, &ContainerFormat::Docx);
            Ok(InspectOutput {
                text_report: None,
                image_report: None,
                meta_findings: findings,
                format,
            })
        }

        MediaHint::Odt => {
            let findings = inspect_file(bytes, &ContainerFormat::Odt);
            Ok(InspectOutput {
                text_report: None,
                image_report: None,
                meta_findings: findings,
                format,
            })
        }

        MediaHint::Html => {
            let findings = inspect_file(bytes, &ContainerFormat::Html);
            Ok(InspectOutput {
                text_report: None,
                image_report: None,
                meta_findings: findings,
                format,
            })
        }

        MediaHint::Markdown => {
            let findings = inspect_file(bytes, &ContainerFormat::Markdown);
            Ok(InspectOutput {
                text_report: None,
                image_report: None,
                meta_findings: findings,
                format,
            })
        }
    }
}

/// Convenience function: inspect and then clean, returning both reports.
///
/// This runs two passes over the input but avoids requiring the caller to
/// call both [`inspect`] and [`clean`] independently.
///
/// # Complexity
/// - Time: O(n): two passes of O(n) each.
/// - Space: O(n).
pub fn inspect_and_clean(
    bytes: &[u8],
    hint: Option<MediaHint>,
) -> Result<(InspectOutput, CleanOutput)> {
    let inspect_out = inspect(bytes, hint.clone())?;
    let clean_out = clean(bytes, hint)?;
    Ok((inspect_out, clean_out))
}

/// Returns a human-readable summary of the detected format.
pub fn format_name(hint: &MediaHint) -> &'static str {
    match hint {
        MediaHint::Text => "plain text",
        MediaHint::Png => "PNG image",
        MediaHint::Jpeg => "JPEG image",
        MediaHint::Webp => "WebP image",
        MediaHint::Svg => "SVG document",
        MediaHint::Pdf => "PDF document",
        MediaHint::Docx => "DOCX document",
        MediaHint::Odt => "ODT document",
        MediaHint::Html => "HTML document",
        MediaHint::Markdown => "Markdown document",
    }
}

/// Returns all [`MetaFinding`] items across both image and container reports in
/// an [`InspectOutput`], flattened into a single vector.
pub fn all_findings(output: &InspectOutput) -> Vec<&MetaFinding> {
    let mut out = Vec::new();
    if let Some(img) = &output.image_report {
        out.extend(img.findings.iter());
    }
    out.extend(output.meta_findings.iter());
    out
}
