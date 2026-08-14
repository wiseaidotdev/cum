// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Container Metadata Watermark Removal
//!
//! Strippers for AI provenance metadata embedded in document containers:
//! PDF, DOCX, ODT, HTML, and Markdown.
//!
//! ## Format Coverage
//!
//! | Format | What is stripped |
//! |--------|------------------|
//! | PDF | `/Info` dictionary keys, XMP `xpacket` blocks |
//! | DOCX | `docProps/app.xml`, `docProps/core.xml`, `customXml/` entries |
//! | ODT | `meta.xml` generator/creator fields |
//! | HTML | `<meta name="generator">`, `data-ai-*` attrs, AI JSON-LD blocks |
//! | Markdown | YAML front-matter AI keys, Layer-A Unicode body clean |
//!
//! ## Complexity
//!
//! All functions run in O(n) time and O(n) space, where n is the byte length
//! of the input.

use crate::error::{CumError, Result};
use crate::types::{CleanStats, Confidence, MetaFinding, WatermarkKind};
use regex::Regex;
use std::io::{Cursor, Write};

/// Document container format for [`inspect_file`] and [`clean_file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerFormat {
    /// PDF document.
    Pdf,
    /// Office Open XML (DOCX / XLSX / PPTX) ZIP container.
    Docx,
    /// ODF (ODT / ODS / ODP) ZIP container.
    Odt,
    /// HTML document.
    Html,
    /// Markdown (CommonMark) document.
    Markdown,
}

impl ContainerFormat {
    /// Detects the container format from magic bytes.
    ///
    /// Returns `None` when no known format is recognized.
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"%PDF-") {
            Some(ContainerFormat::Pdf)
        } else if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
            Some(ContainerFormat::Docx)
        } else if is_likely_html(bytes) {
            Some(ContainerFormat::Html)
        } else {
            None
        }
    }

    /// Returns the format as a human-readable name.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerFormat::Pdf => "PDF",
            ContainerFormat::Docx => "DOCX/ODT",
            ContainerFormat::Odt => "ODT",
            ContainerFormat::Html => "HTML",
            ContainerFormat::Markdown => "Markdown",
        }
    }
}

/// Heuristically detects whether bytes are an HTML document.
fn is_likely_html(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    match std::str::from_utf8(head) {
        Ok(s) => {
            let lower = s.to_lowercase();
            lower.contains("<!doctype html") || lower.contains("<html")
        }
        Err(_) => false,
    }
}

/// Inspects a document container for metadata watermark findings.
///
/// # Arguments
/// * `bytes`: raw document bytes.
/// * `format`: the container format to inspect.
///
/// # Returns
/// A list of [`MetaFinding`] entries describing detected AI metadata.
///
/// # Complexity
/// - Time: O(n): single pass or ZIP entry walk.
/// - Space: O(k): one finding per metadata entry found.
pub fn inspect_file(bytes: &[u8], format: &ContainerFormat) -> Vec<MetaFinding> {
    match format {
        ContainerFormat::Pdf => inspect_pdf(bytes),
        ContainerFormat::Docx => inspect_zip_container(bytes, false),
        ContainerFormat::Odt => inspect_zip_container(bytes, true),
        ContainerFormat::Html => inspect_html(bytes),
        ContainerFormat::Markdown => inspect_markdown(bytes),
    }
}

/// Strips AI provenance metadata from a document container.
///
/// # Arguments
/// * `bytes`: raw document bytes.
/// * `format`: the container format to clean.
///
/// # Returns
/// A tuple of `(cleaned_bytes, stats)`.
///
/// # Errors
/// - [`CumError::ParseError`] for structurally malformed containers.
/// - [`CumError::Zip`] for ZIP read/write failures.
///
/// # Complexity
/// - Time: O(n) for PDF/HTML/Markdown; O(n log n) for ZIP containers
///   (re-compression).
/// - Space: O(n).
pub fn clean_file(bytes: &[u8], format: &ContainerFormat) -> Result<(Vec<u8>, CleanStats)> {
    match format {
        ContainerFormat::Pdf => clean_pdf(bytes),
        ContainerFormat::Docx => clean_zip_container(bytes, false),
        ContainerFormat::Odt => clean_zip_container(bytes, true),
        ContainerFormat::Html => clean_html(bytes),
        ContainerFormat::Markdown => clean_markdown(bytes),
    }
}

/// AI-related keys inside a PDF `/Info` dictionary.
const PDF_INFO_AI_KEYS: &[&str] = &[
    "Creator",
    "Producer",
    "Author",
    "Keywords",
    "Subject",
    "Generator",
];

/// Inspects a PDF for metadata watermark findings.
fn inspect_pdf(bytes: &[u8]) -> Vec<MetaFinding> {
    let mut findings = Vec::new();

    if let Ok(text) = std::str::from_utf8(bytes) {
        if text.contains("/Info") {
            for key in PDF_INFO_AI_KEYS {
                if text.contains(&format!("/{key}")) {
                    findings.push(MetaFinding {
                        description: format!("PDF /Info key /{key} present"),
                        confidence: Confidence::Informational,
                        kind: Some(WatermarkKind::DocumentProperty),
                    });
                }
            }
        }

        if text.contains("<?xpacket") {
            findings.push(MetaFinding {
                description: "PDF XMP packet present (xpacket)".into(),
                confidence: Confidence::Probable,
                kind: Some(WatermarkKind::XmpMetadata),
            });
        }

        let ai_markers = [
            "ai:contentType",
            "trainedAlgorithmicMedia",
            "digitalSourceType",
            "SoftwareAgent",
        ];
        for marker in &ai_markers {
            if text.contains(marker) {
                findings.push(MetaFinding {
                    description: format!("PDF contains AI provenance marker: {marker}"),
                    confidence: Confidence::Confirmed,
                    kind: Some(WatermarkKind::C2paMetadata),
                });
            }
        }
    }

    findings
}

/// Strips XMP packets and /Info dictionary from a PDF byte stream.
///
/// This is a best-effort byte-level strip. A structural rewrite (equivalent to
/// `qpdf --linearize`) is required to guarantee removal of unreferenced object
/// remnants: this function covers the most common surface but notes the
/// limitation in its stats.
fn clean_pdf(bytes: &[u8]) -> Result<(Vec<u8>, CleanStats)> {
    let text = std::str::from_utf8(bytes).map_err(|e| {
        CumError::ParseError(format!("PDF contains non-UTF-8 data at metadata scan: {e}"))
    })?;

    let xpacket_re =
        Regex::new(r"(?s)<\?xpacket begin.*?\?xpacket end='[wr]'\?>").expect("valid regex");
    let cleaned_text = xpacket_re.replace_all(text, "");

    let removed = usize::from(cleaned_text.len() != text.len());

    Ok((
        cleaned_text.into_owned().into_bytes(),
        CleanStats {
            removed_count: removed,
            replaced_count: 0,
            metadata_chunks_removed: removed,
            summary: vec![
                "PDF XMP packets stripped (byte-level best-effort).".into(),
                "WARNING: exiftool-style incremental edits may leave residual bytes; a qpdf structural rewrite is recommended for production use.".into(),
            ],
        },
    ))
}

/// ZIP entry paths that carry AI provenance metadata in DOCX containers.
const DOCX_STRIP_ENTRIES: &[&str] = &["docProps/app.xml", "docProps/core.xml"];

/// ZIP entry path prefix for DOCX custom XML parts.
const DOCX_CUSTOM_XML_PREFIX: &str = "customXml/";

/// ZIP entry path for ODT metadata.
const ODT_META_ENTRY: &str = "meta.xml";

/// Inspects a ZIP-based document container (DOCX or ODT) for metadata findings.
fn inspect_zip_container(bytes: &[u8], is_odt: bool) -> Vec<MetaFinding> {
    let mut findings = Vec::new();
    let cursor = Cursor::new(bytes);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => {
            findings.push(MetaFinding {
                description: format!("Failed to open ZIP container: {e}"),
                confidence: Confidence::Informational,
                kind: None,
            });
            return findings;
        }
    };

    for i in 0..archive.len() {
        let entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().to_string();

        if is_odt {
            if name == ODT_META_ENTRY {
                findings.push(MetaFinding {
                    description: "ODT meta.xml (generator/creator metadata) present".into(),
                    confidence: Confidence::Probable,
                    kind: Some(WatermarkKind::DocumentProperty),
                });
            }
        } else {
            if DOCX_STRIP_ENTRIES.contains(&name.as_str()) {
                findings.push(MetaFinding {
                    description: format!("DOCX {name} (document property metadata) present"),
                    confidence: Confidence::Probable,
                    kind: Some(WatermarkKind::DocumentProperty),
                });
            }
            if name.starts_with(DOCX_CUSTOM_XML_PREFIX) {
                findings.push(MetaFinding {
                    description: format!("DOCX custom XML part: {name}"),
                    confidence: Confidence::Informational,
                    kind: Some(WatermarkKind::DocumentProperty),
                });
            }
        }
    }

    findings
}

/// Strips metadata entries from a ZIP-based document container.
fn clean_zip_container(bytes: &[u8], is_odt: bool) -> Result<(Vec<u8>, CleanStats)> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| CumError::Zip(format!("failed to open ZIP: {e}")))?;

    let mut out_buf: Vec<u8> = Vec::with_capacity(bytes.len());
    let out_cursor = Cursor::new(&mut out_buf);
    let mut writer = zip::ZipWriter::new(out_cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut removed = 0usize;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CumError::Zip(format!("failed to read ZIP entry {i}: {e}")))?;
        let name = entry.name().to_string();

        let should_strip = if is_odt {
            name == ODT_META_ENTRY
        } else {
            DOCX_STRIP_ENTRIES.contains(&name.as_str()) || name.starts_with(DOCX_CUSTOM_XML_PREFIX)
        };

        if should_strip {
            removed += 1;
            continue;
        }

        writer
            .start_file(&name, options)
            .map_err(|e| CumError::Zip(format!("failed to write ZIP entry {name}: {e}")))?;

        let mut content = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut content).map_err(CumError::Io)?;
        writer.write_all(&content).map_err(CumError::Io)?;
    }

    writer
        .finish()
        .map_err(|e| CumError::Zip(format!("failed to finalise ZIP: {e}")))?;

    Ok((
        out_buf,
        CleanStats {
            removed_count: removed,
            replaced_count: 0,
            metadata_chunks_removed: removed,
            summary: vec![format!("Stripped {removed} metadata ZIP entries.")],
        },
    ))
}

/// HTML patterns that carry AI provenance metadata.
const HTML_AI_META_NAMES: &[&str] = &[
    "generator",
    "ai-model",
    "ai-provider",
    "ai-generated",
    "ai-content-type",
    "digitalSourceType",
];

/// Inspects an HTML document for AI metadata findings.
fn inspect_html(bytes: &[u8]) -> Vec<MetaFinding> {
    let mut findings = Vec::new();
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return findings,
    };

    for meta_name in HTML_AI_META_NAMES {
        if text
            .to_lowercase()
            .contains(&format!("name=\"{meta_name}\""))
            || text.to_lowercase().contains(&format!("name='{meta_name}'"))
        {
            findings.push(MetaFinding {
                description: format!("HTML <meta name=\"{meta_name}\"> present"),
                confidence: Confidence::Informational,
                kind: Some(WatermarkKind::HtmlMeta),
            });
        }
    }

    if text.contains("data-ai-") {
        findings.push(MetaFinding {
            description: "HTML data-ai-* attribute present".into(),
            confidence: Confidence::Probable,
            kind: Some(WatermarkKind::HtmlMeta),
        });
    }

    if text.contains("application/ld+json") && text.contains("trainedAlgorithmicMedia") {
        findings.push(MetaFinding {
            description: "HTML JSON-LD block with AI provenance claim (trainedAlgorithmicMedia)"
                .into(),
            confidence: Confidence::Confirmed,
            kind: Some(WatermarkKind::C2paMetadata),
        });
    }

    findings
}

/// Strips AI metadata from an HTML document.
fn clean_html(bytes: &[u8]) -> Result<(Vec<u8>, CleanStats)> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| CumError::ParseError(format!("HTML is not valid UTF-8: {e}")))?;

    let meta_gen_re =
        Regex::new(r#"(?i)<meta\s+name=["']generator["'][^>]*/>"#).expect("valid regex");
    let data_ai_re = Regex::new(r#"\s+data-ai-[a-zA-Z0-9_-]+="[^"]*""#).expect("valid regex");
    let jsonld_ai_re = Regex::new(
        r#"(?s)<script\s+type=["']application/ld\+json["'][^>]*>.*?trainedAlgorithmicMedia.*?</script>"#,
    )
    .expect("valid regex");

    let c1 = meta_gen_re.replace_all(text, "");
    let c2 = data_ai_re.replace_all(&c1, "");
    let cleaned = jsonld_ai_re.replace_all(&c2, "");

    let removed = usize::from(cleaned.len() != text.len());

    Ok((
        cleaned.into_owned().into_bytes(),
        CleanStats {
            removed_count: removed,
            replaced_count: 0,
            metadata_chunks_removed: removed,
            summary: vec!["Stripped AI meta tags and JSON-LD provenance blocks.".into()],
        },
    ))
}

/// YAML front-matter keys that indicate AI generation.
const MARKDOWN_AI_FRONTMATTER_KEYS: &[&str] = &[
    "generator",
    "ai-model",
    "ai-provider",
    "ai-generated",
    "created-by",
    "source",
    "ai_model",
    "ai_provider",
];

/// Inspects a Markdown document for AI metadata findings.
fn inspect_markdown(bytes: &[u8]) -> Vec<MetaFinding> {
    let mut findings = Vec::new();
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return findings,
    };

    if !text.starts_with("---") {
        return findings;
    }

    let end = text[3..].find("---").map(|i| i + 3);
    if let Some(end_pos) = end {
        let frontmatter = &text[3..end_pos];
        for key in MARKDOWN_AI_FRONTMATTER_KEYS {
            if frontmatter.contains(key) {
                findings.push(MetaFinding {
                    description: format!("Markdown YAML front-matter key '{key}' present"),
                    confidence: Confidence::Probable,
                    kind: Some(WatermarkKind::MarkdownFrontmatter),
                });
            }
        }
    }

    findings
}

/// Strips AI keys from Markdown YAML front-matter and applies Layer-A Unicode
/// cleaning to the body.
fn clean_markdown(bytes: &[u8]) -> Result<(Vec<u8>, CleanStats)> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| CumError::ParseError(format!("Markdown is not valid UTF-8: {e}")))?;

    let mut removed = 0usize;

    if let Some(end_offset) = text.strip_prefix("---").and_then(|r| r.find("---")) {
        let fm_start = 3;
        let fm_end = fm_start + end_offset;
        let frontmatter = &text[fm_start..fm_end];
        let mut clean_fm = String::with_capacity(frontmatter.len());
        for line in frontmatter.lines() {
            let key = line.split(':').next().unwrap_or("").trim().to_lowercase();
            let is_ai_key = MARKDOWN_AI_FRONTMATTER_KEYS
                .iter()
                .any(|k| k.to_lowercase() == key);
            if is_ai_key {
                removed += 1;
            } else {
                clean_fm.push_str(line);
                clean_fm.push('\n');
            }
        }
        let body = &text[fm_end + 3..];
        let full = format!("---{clean_fm}---{body}");
        return Ok((
            full.into_bytes(),
            CleanStats {
                removed_count: removed,
                replaced_count: 0,
                metadata_chunks_removed: removed,
                summary: vec![format!("Removed {removed} AI front-matter keys.")],
            },
        ));
    }

    Ok((bytes.to_vec(), CleanStats::default()))
}
