// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Image Metadata Watermark Removal
//!
//! Pure-Rust strippers for C2PA, EXIF, and XMP provenance metadata embedded
//! in PNG, JPEG, WebP, and SVG images.
//!
//! ## Format Coverage
//!
//! | Format | Stripped chunks / segments |
//! |--------|-----------------------------|
//! | PNG | `iTXt`, `tEXt`, `zTXt`, `eXIf`, `C2PA`, `caBX`, `JUMB`, `JUMD` |
//! | JPEG | `APP1` (EXIF/XMP), `APP11` (JUMBF C2PA), `APP13` (IPTC/Photoshop) |
//! | WebP | RIFF `EXIF`, `XMP `, `ICCP`, `C2PA` sub-chunks in `VP8X` container |
//! | SVG | `<metadata>`, `<x:xmpmeta>`, `<rdf:RDF>`, `data-ai-*` attributes |
//!
//! ## Approach
//!
//! Each format is decoded at the byte/chunk level: no third-party image
//! library is required.  The cleaned byte stream is rebuilt from scratch
//! so no residual metadata objects remain (unlike tools that write
//! incremental updates).
//!
//! ## Complexity
//!
//! All functions run in O(n) time and O(n) space, where n is the number of
//! input bytes.

use crate::error::{CumError, Result};
use crate::types::{Confidence, ImageFormat, ImageInspectReport, MetaFinding, WatermarkKind};
use regex::Regex;

/// PNG magic bytes: `\x89PNG\r\n\x1A\n`.
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// JPEG magic bytes: `\xFF\xD8\xFF`.
const JPEG_MAGIC: &[u8] = b"\xFF\xD8\xFF";

/// WebP RIFF magic: first 4 bytes `RIFF`, bytes 8-11 `WEBP`.
const RIFF_MAGIC: &[u8] = b"RIFF";
const WEBP_ID: &[u8] = b"WEBP";

/// PNG chunk types that carry AI provenance metadata and must be stripped.
const PNG_STRIP_CHUNKS: &[&[u8; 4]] = &[
    b"iTXt", b"tEXt", b"zTXt", b"eXIf", b"C2PA", b"caBX", b"JUMB", b"JUMD",
];

/// JPEG application-layer markers whose segments carry provenance metadata.
///
/// `0xE1` = APP1 (EXIF + XMP)
/// `0xEB` = APP11 (JUMBF / C2PA)
/// `0xED` = APP13 (IPTC / Photoshop IRB)
const JPEG_STRIP_MARKERS: &[u8] = &[0xE1, 0xEB, 0xED];

/// WebP RIFF sub-chunk FourCCs that carry image metadata.
const WEBP_STRIP_CHUNKS: &[&[u8; 4]] = &[b"EXIF", b"XMP ", b"ICCP", b"C2PA"];

/// Detects the image format from a magic-byte prefix.
pub fn detect_image_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(PNG_MAGIC) {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(JPEG_MAGIC) {
        Some(ImageFormat::Jpeg)
    } else if bytes.starts_with(RIFF_MAGIC) && bytes.get(8..12) == Some(WEBP_ID) {
        Some(ImageFormat::Webp)
    } else if is_likely_svg(bytes) {
        Some(ImageFormat::Svg)
    } else {
        None
    }
}

/// Heuristically decides whether raw bytes are likely SVG (UTF-8 XML
/// containing a `<svg` opening tag within the first 2 KiB).
fn is_likely_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(2048)];
    std::str::from_utf8(head)
        .map(|s| s.contains("<svg") || s.contains("<!DOCTYPE svg"))
        .unwrap_or(false)
}

/// Inspects an image for metadata watermark findings without modifying it.
///
/// # Arguments
/// * `bytes`: raw image bytes.
///
/// # Returns
/// An [`ImageInspectReport`] listing every metadata chunk or attribute found.
///
/// # Errors
/// Returns [`CumError::UnsupportedFormat`] if the format cannot be detected.
///
/// # Complexity
/// - Time: O(n): single pass over the byte stream.
/// - Space: O(k): one finding entry per metadata chunk found.
pub fn inspect_image(bytes: &[u8]) -> Result<ImageInspectReport> {
    match detect_image_format(bytes) {
        Some(ImageFormat::Png) => Ok(ImageInspectReport {
            format: ImageFormat::Png,
            findings: inspect_png(bytes),
        }),
        Some(ImageFormat::Jpeg) => Ok(ImageInspectReport {
            format: ImageFormat::Jpeg,
            findings: inspect_jpeg(bytes),
        }),
        Some(ImageFormat::Webp) => Ok(ImageInspectReport {
            format: ImageFormat::Webp,
            findings: inspect_webp(bytes),
        }),
        Some(ImageFormat::Svg) => Ok(ImageInspectReport {
            format: ImageFormat::Svg,
            findings: inspect_svg(bytes),
        }),
        None => Err(CumError::UnsupportedFormat(
            "unrecognised image format (expected PNG, JPEG, WebP, or SVG)".into(),
        )),
    }
}

/// Strips all provenance metadata from an image and returns the cleaned bytes.
///
/// # Arguments
/// * `bytes`: raw image bytes.
///
/// # Returns
/// A `Vec<u8>` containing the reconstructed image with all metadata removed.
///
/// # Errors
/// - [`CumError::UnsupportedFormat`] if the format is not recognised.
/// - [`CumError::ParseError`] if the byte stream is structurally malformed.
///
/// # Complexity
/// - Time: O(n): single pass over the byte stream.
/// - Space: O(n): output buffer pre-allocated at `bytes.len()`.
pub fn clean_image(bytes: &[u8]) -> Result<Vec<u8>> {
    match detect_image_format(bytes) {
        Some(ImageFormat::Png) => clean_png(bytes),
        Some(ImageFormat::Jpeg) => clean_jpeg(bytes),
        Some(ImageFormat::Webp) => clean_webp(bytes),
        Some(ImageFormat::Svg) => clean_svg(bytes),
        None => Err(CumError::UnsupportedFormat(
            "unrecognised image format (expected PNG, JPEG, WebP, or SVG)".into(),
        )),
    }
}

/// Reads a big-endian u32 from a byte slice at the given offset.
fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Reads a little-endian u32 from a byte slice at the given offset.
fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Classifies a PNG chunk type into a [`WatermarkKind`] if it carries metadata.
fn png_chunk_kind(chunk_type: &[u8; 4]) -> Option<WatermarkKind> {
    match chunk_type {
        b"C2PA" | b"caBX" | b"JUMB" | b"JUMD" => Some(WatermarkKind::C2paMetadata),
        b"eXIf" => Some(WatermarkKind::ExifMetadata),
        b"iTXt" | b"tEXt" | b"zTXt" => Some(WatermarkKind::XmpMetadata),
        _ => None,
    }
}

/// Confidence assigned to a detected PNG metadata chunk.
fn png_chunk_confidence(chunk_type: &[u8; 4]) -> Confidence {
    match chunk_type {
        b"C2PA" | b"caBX" | b"JUMB" | b"JUMD" => Confidence::Confirmed,
        b"eXIf" => Confidence::Probable,
        _ => Confidence::Informational,
    }
}

/// Walks PNG chunks and returns findings for all metadata chunks present.
fn inspect_png(bytes: &[u8]) -> Vec<MetaFinding> {
    let mut findings = Vec::new();
    let mut pos = PNG_MAGIC.len();

    while pos + 12 <= bytes.len() {
        let length = match read_u32_be(bytes, pos) {
            Some(l) => l as usize,
            None => break,
        };
        let chunk_type: [u8; 4] = match bytes.get(pos + 4..pos + 8) {
            Some(t) => [t[0], t[1], t[2], t[3]],
            None => break,
        };

        let chunk_end = pos + 4 + 4 + length + 4;
        if chunk_end > bytes.len() {
            findings.push(MetaFinding {
                description: format!(
                    "Truncated chunk {} at offset {}",
                    String::from_utf8_lossy(&chunk_type),
                    pos
                ),
                confidence: Confidence::Informational,
                kind: None,
            });
            break;
        }

        for &strip_type in PNG_STRIP_CHUNKS {
            if chunk_type == *strip_type {
                let kind = png_chunk_kind(strip_type);
                let confidence = png_chunk_confidence(strip_type);
                findings.push(MetaFinding {
                    description: format!(
                        "PNG chunk {} (potential AI provenance metadata)",
                        String::from_utf8_lossy(&chunk_type)
                    ),
                    confidence,
                    kind,
                });
            }
        }

        if chunk_type == *b"IEND" {
            break;
        }

        pos = chunk_end;
    }

    findings
}

/// Strips all provenance metadata chunks from a PNG byte stream.
fn clean_png(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(PNG_MAGIC);

    let mut pos = PNG_MAGIC.len();

    while pos + 12 <= bytes.len() {
        let length = read_u32_be(bytes, pos)
            .ok_or_else(|| CumError::ParseError("truncated PNG chunk length".into()))?
            as usize;

        let chunk_type: [u8; 4] = bytes
            .get(pos + 4..pos + 8)
            .map(|t| [t[0], t[1], t[2], t[3]])
            .ok_or_else(|| CumError::ParseError("truncated PNG chunk type".into()))?;

        let chunk_end = pos + 4 + 4 + length + 4;
        if chunk_end > bytes.len() {
            return Err(CumError::ParseError(format!(
                "PNG chunk {} extends past end of file",
                String::from_utf8_lossy(&chunk_type)
            )));
        }

        let should_strip = PNG_STRIP_CHUNKS.iter().any(|&t| chunk_type == *t);

        if !should_strip {
            out.extend_from_slice(&bytes[pos..chunk_end]);
        }

        if chunk_type == *b"IEND" {
            break;
        }

        pos = chunk_end;
    }

    Ok(out)
}

/// Walks JPEG segments and returns findings for all metadata segments.
fn inspect_jpeg(bytes: &[u8]) -> Vec<MetaFinding> {
    let mut findings = Vec::new();
    let mut pos = 2;

    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }
        let marker = bytes[pos + 1];

        if marker == 0xD9 || marker == 0xDA {
            break;
        }

        let seg_len = match bytes.get(pos + 2..pos + 4) {
            Some(b) => u16::from_be_bytes([b[0], b[1]]) as usize,
            None => break,
        };

        if JPEG_STRIP_MARKERS.contains(&marker) {
            let (kind, confidence, desc) = jpeg_marker_meta(marker, bytes, pos + 4, seg_len);
            findings.push(MetaFinding {
                description: desc,
                confidence,
                kind: Some(kind),
            });
        }

        pos += 2 + seg_len;
    }

    findings
}

/// Returns the kind, confidence, and description for a JPEG marker.
fn jpeg_marker_meta(
    marker: u8,
    bytes: &[u8],
    data_start: usize,
    seg_len: usize,
) -> (WatermarkKind, Confidence, String) {
    match marker {
        0xE1 => {
            let data = bytes.get(data_start..data_start + seg_len.saturating_sub(2));
            if data.map(|d| d.starts_with(b"Exif\0\0")).unwrap_or(false) {
                (
                    WatermarkKind::ExifMetadata,
                    Confidence::Probable,
                    "JPEG APP1 (EXIF metadata)".into(),
                )
            } else {
                (
                    WatermarkKind::XmpMetadata,
                    Confidence::Probable,
                    "JPEG APP1 (XMP metadata)".into(),
                )
            }
        }
        0xEB => (
            WatermarkKind::C2paMetadata,
            Confidence::Confirmed,
            "JPEG APP11 (JUMBF / C2PA provenance manifest)".into(),
        ),
        0xED => (
            WatermarkKind::DocumentProperty,
            Confidence::Informational,
            "JPEG APP13 (IPTC / Photoshop IRB)".into(),
        ),
        _ => (
            WatermarkKind::ExifMetadata,
            Confidence::Informational,
            format!("JPEG APPx segment (marker=0x{marker:02X})"),
        ),
    }
}

/// Strips all provenance metadata segments from a JPEG byte stream.
fn clean_jpeg(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..2]);

    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            out.extend_from_slice(&bytes[pos..]);
            break;
        }

        let marker = bytes[pos + 1];

        if marker == 0xD9 || marker == 0xDA {
            out.extend_from_slice(&bytes[pos..]);
            break;
        }

        let seg_len = bytes
            .get(pos + 2..pos + 4)
            .map(|b| u16::from_be_bytes([b[0], b[1]]) as usize)
            .ok_or_else(|| CumError::ParseError("truncated JPEG segment".into()))?;

        let seg_end = pos + 2 + seg_len;
        if seg_end > bytes.len() {
            return Err(CumError::ParseError(
                "JPEG segment extends past end of file".into(),
            ));
        }

        let should_strip = JPEG_STRIP_MARKERS.contains(&marker);

        if !should_strip {
            out.extend_from_slice(&bytes[pos..seg_end]);
        }

        pos = seg_end;
    }

    // The JPEG SOI loop only enters when `pos + 4 <= bytes.len()`, so a
    // standalone 2-byte EOI (0xFF 0xD9) at the end of the stream is never
    // processed inside the loop.  Append whatever unconsumed tail remains.
    if pos < bytes.len() {
        out.extend_from_slice(&bytes[pos..]);
    }

    Ok(out)
}

/// Walks a WebP RIFF container and returns findings for metadata sub-chunks.
fn inspect_webp(bytes: &[u8]) -> Vec<MetaFinding> {
    let mut findings = Vec::new();

    let total_riff_len = match read_u32_le(bytes, 4) {
        Some(l) => l as usize,
        None => return findings,
    };

    let file_end = (4 + 4 + total_riff_len).min(bytes.len());
    let mut pos = 12;

    while pos + 8 <= file_end {
        let chunk_id: [u8; 4] = match bytes.get(pos..pos + 4) {
            Some(b) => [b[0], b[1], b[2], b[3]],
            None => break,
        };
        let chunk_size = match read_u32_le(bytes, pos + 4) {
            Some(s) => s as usize,
            None => break,
        };

        for &strip_id in WEBP_STRIP_CHUNKS {
            if chunk_id == *strip_id {
                let kind = match &chunk_id {
                    b"EXIF" => WatermarkKind::ExifMetadata,
                    b"XMP " => WatermarkKind::XmpMetadata,
                    b"C2PA" => WatermarkKind::C2paMetadata,
                    _ => WatermarkKind::XmpMetadata,
                };
                let confidence = match &chunk_id {
                    b"C2PA" => Confidence::Confirmed,
                    _ => Confidence::Probable,
                };
                findings.push(MetaFinding {
                    description: format!(
                        "WebP RIFF chunk {} (potential AI provenance metadata)",
                        String::from_utf8_lossy(&chunk_id)
                    ),
                    confidence,
                    kind: Some(kind),
                });
            }
        }

        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }

    findings
}

/// Strips all provenance metadata sub-chunks from a WebP RIFF byte stream.
fn clean_webp(bytes: &[u8]) -> Result<Vec<u8>> {
    let total_riff_len = read_u32_le(bytes, 4)
        .ok_or_else(|| CumError::ParseError("short WebP RIFF header".into()))?
        as usize;

    let file_end = (4 + 4 + total_riff_len).min(bytes.len());

    let mut kept_chunks: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut pos = 12;

    while pos + 8 <= file_end {
        let chunk_id: [u8; 4] = bytes
            .get(pos..pos + 4)
            .map(|b| [b[0], b[1], b[2], b[3]])
            .ok_or_else(|| CumError::ParseError("truncated WebP chunk id".into()))?;

        let chunk_size = read_u32_le(bytes, pos + 4)
            .ok_or_else(|| CumError::ParseError("truncated WebP chunk size".into()))?
            as usize;

        let padded = chunk_size + (chunk_size & 1);
        let chunk_end = pos + 8 + padded;

        let should_strip = WEBP_STRIP_CHUNKS.iter().any(|&t| chunk_id == *t);
        if !should_strip {
            kept_chunks.extend_from_slice(&bytes[pos..chunk_end.min(file_end)]);
        }

        pos = chunk_end;
    }

    let new_total = 4 + kept_chunks.len();
    let mut out = Vec::with_capacity(4 + 4 + new_total);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(new_total as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&kept_chunks);

    Ok(out)
}

/// Walks SVG text for XMP/metadata blocks and returns findings.
fn inspect_svg(bytes: &[u8]) -> Vec<MetaFinding> {
    let mut findings = Vec::new();
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return findings,
    };

    let patterns = [
        (
            "<metadata",
            WatermarkKind::XmpMetadata,
            "SVG <metadata> block present",
        ),
        (
            "<x:xmpmeta",
            WatermarkKind::XmpMetadata,
            "SVG XMP metadata (x:xmpmeta) present",
        ),
        (
            "<rdf:RDF",
            WatermarkKind::XmpMetadata,
            "SVG RDF/XMP block present",
        ),
        (
            "data-ai-",
            WatermarkKind::HtmlMeta,
            "SVG data-ai-* attribute present",
        ),
        (
            "ai:contentType",
            WatermarkKind::HtmlMeta,
            "SVG ai:contentType attribute present",
        ),
    ];

    for (pattern, kind, desc) in &patterns {
        if text.contains(pattern) {
            findings.push(MetaFinding {
                description: (*desc).to_string(),
                confidence: if *pattern == "data-ai-" || *pattern == "ai:contentType" {
                    Confidence::Probable
                } else {
                    Confidence::Informational
                },
                kind: Some(kind.clone()),
            });
        }
    }

    findings
}

/// Strips XMP metadata blocks and AI attributes from SVG text.
fn clean_svg(bytes: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| CumError::ParseError(format!("SVG is not valid UTF-8: {e}")))?;

    let meta_re = Regex::new(r"(?s)<metadata\b[^>]*>.*?</metadata>").expect("valid regex");
    let xmpmeta_re = Regex::new(r"(?s)<x:xmpmeta\b[^>]*>.*?</x:xmpmeta>").expect("valid regex");
    let rdf_re = Regex::new(r"(?s)<rdf:RDF\b[^>]*>.*?</rdf:RDF>").expect("valid regex");
    let data_ai_re = Regex::new(r#"\s+data-ai-[a-zA-Z0-9_-]+="[^"]*""#).expect("valid regex");
    let ai_attr_re = Regex::new(r#"\s+ai:[a-zA-Z0-9_-]+="[^"]*""#).expect("valid regex");

    let cleaned = meta_re.replace_all(text, "");
    let cleaned = xmpmeta_re.replace_all(&cleaned, "");
    let cleaned = rdf_re.replace_all(&cleaned, "");
    let cleaned = data_ai_re.replace_all(&cleaned, "");
    let cleaned = ai_attr_re.replace_all(&cleaned, "");

    Ok(cleaned.into_owned().into_bytes())
}
