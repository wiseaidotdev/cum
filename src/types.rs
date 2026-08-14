// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Public Types
//!
//! Strongly-typed enumerations and data structures used throughout the crate.
//!
//! ## Primary Types
//!
//! | Type | Description |
//! |------|-------------|
//! | [`Provider`] | LLM provider that embedded the watermark. |
//! | [`WatermarkKind`] | Specific technique used to embed the mark. |
//! | [`Confidence`] | How certain the detection is. |
//! | [`MediaHint`] | Caller-provided or auto-detected media format. |
//! | [`CharHit`] | A single Layer-A Unicode carrier finding. |
//! | [`MetaFinding`] | A single file-metadata watermark finding. |
//! | [`TextInspectReport`] | Full Layer-A inspection result for a text string. |
//! | [`ImageInspectReport`] | Full metadata inspection result for an image. |
//! | [`CleanStats`] | Counts of removed / replaced marks after cleaning. |
//! | [`CleanOutput`] | Cleaned bytes + statistics returned by `clean()`. |
//! | [`InspectOutput`] | Findings returned by `inspect()`. |

use serde::{Deserialize, Serialize};

/// The LLM provider suspected of embedding a watermark.
///
/// This is a best-effort classification; many Unicode and metadata marks are
/// vendor-agnostic. Use [`Provider::Unknown`] when the vendor cannot be
/// determined from the mark alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    /// Anthropic Claude (text watermarks, C2PA provenance metadata).
    Claude,
    /// OpenAI models (provenance metadata, possible token-bias marks).
    OpenAi,
    /// Google Gemini / SynthID-Text (statistical token-sampling watermarks).
    Gemini,
    /// xAI Grok (provenance metadata).
    Grok,
    /// Open-source LLMs using Kirchenbauer-style KGW marks.
    OpenLlm,
    /// The vendor could not be determined from available signals.
    Unknown,
}

impl Provider {
    /// Returns a human-readable display name for the provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Claude => "Claude (Anthropic)",
            Provider::OpenAi => "OpenAI",
            Provider::Gemini => "Gemini / SynthID (Google)",
            Provider::Grok => "Grok (xAI)",
            Provider::OpenLlm => "Open-LLM (KGW-class)",
            Provider::Unknown => "Unknown",
        }
    }
}

/// The specific technique used to embed the watermark.
///
/// Maps to the three detection layers described in the `watermarks-remover`
/// reference project:
///
/// - Layer A (deterministic): Unicode invisible carriers, space homoglyphs,
///   confusable Latin substitutions.
/// - Layer File: C2PA, EXIF, XMP, and document-property metadata.
/// - Layer Pixel: SynthID-class pixel-domain marks (detection only; removal
///   is external/best-effort).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatermarkKind {
    /// Invisible Unicode control character (ZWSP, bidi controls, word joiner,
    /// function application, etc.).
    UnicodeCarrier,
    /// Unicode space homoglyph (en-space, hair-space, narrow NBSP, etc.)
    /// substituted for a regular space.
    SpaceHomoglyph,
    /// Cyrillic or fullwidth Latin confusable substituted for an ASCII letter.
    LatinConfusable,
    /// Unicode tag character in the range U+E0001-U+E007F (used in some
    /// steganography schemes; also appears legitimately in flag emoji).
    TagChar,
    /// Variation selector attached to a character where none is orthographically
    /// required.
    VariationSelector,
    /// Bidirectional format control (LRE, RLE, LRO, RLO, PDF, LRI, RLI, FSI,
    /// PDI, LRM, RLM, ALM).
    Bidi,
    /// Zero-width character family (ZWSP, ZWNJ, ZWJ, WJ, BOM/ZWNBSP, VS
    /// Mongolian separator).
    ZwjFamily,
    /// Character in a Unicode private-use area (BMP PUA U+E000-F8FF,
    /// Supplementary PUA-A/B).
    PrivateUse,
    /// C2PA / JUMBF provenance manifest in a file's metadata.
    C2paMetadata,
    /// EXIF metadata block containing AI-generator fields.
    ExifMetadata,
    /// XMP packet containing AI-model or generator fields.
    XmpMetadata,
    /// Statistical pixel-domain watermark (SynthID-class, StegaStamp,
    /// Tree-Ring, StableSignature).
    PixelDomain,
    /// Document property or custom XML metadata (DOCX `docProps/`, ODT
    /// `meta.xml`, PDF `/Info` dict).
    DocumentProperty,
    /// HTML or JSON-LD metadata attribute indicating AI generation.
    HtmlMeta,
    /// YAML front-matter key in a Markdown file indicating AI generation.
    MarkdownFrontmatter,
}

impl WatermarkKind {
    /// Returns a short human-readable label for the watermark kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            WatermarkKind::UnicodeCarrier => "unicode_carrier",
            WatermarkKind::SpaceHomoglyph => "space_homoglyph",
            WatermarkKind::LatinConfusable => "latin_confusable",
            WatermarkKind::TagChar => "tag_chars",
            WatermarkKind::VariationSelector => "variation_selector",
            WatermarkKind::Bidi => "bidi",
            WatermarkKind::ZwjFamily => "zwj_family",
            WatermarkKind::PrivateUse => "private_use",
            WatermarkKind::C2paMetadata => "c2pa_metadata",
            WatermarkKind::ExifMetadata => "exif_metadata",
            WatermarkKind::XmpMetadata => "xmp_metadata",
            WatermarkKind::PixelDomain => "pixel_domain",
            WatermarkKind::DocumentProperty => "document_property",
            WatermarkKind::HtmlMeta => "html_meta",
            WatermarkKind::MarkdownFrontmatter => "markdown_frontmatter",
        }
    }
}

/// Confidence level of a watermark detection finding.
///
/// Follows the same four-bucket model as the `watermarks-remover` reference:
///
/// | Level | Meaning |
/// |-------|---------|
/// | [`Confidence::Confirmed`] | Fully parsed provenance structure found. |
/// | [`Confidence::Probable`] | AI marker in a recognised metadata structure. |
/// | [`Confidence::Informational`] | Context-only signal; not conclusive. |
/// | [`Confidence::LikelyFalsePositive`] | Raw byte pattern; high collision risk. |
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// A known provenance structure (C2PA manifest, `digitalSourceType`,
    /// `trainedAlgorithmicMedia`) was found and parsed.
    Confirmed,
    /// An AI/vendor marker was found inside a recognised metadata structure
    /// (XMP packet, EXIF field) but the full provenance chain was not parsed.
    Probable,
    /// Context-only signal: e.g. a CMS generator tag, presence of a custom
    /// XML part, or an unsupported path. Not conclusive evidence of AI origin.
    Informational,
    /// A raw full-file byte scan matched an AI keyword. Compression artefacts
    /// and binary-format coincidences produce many false positives here.
    LikelyFalsePositive,
}

impl Confidence {
    /// Returns the confidence level as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::Confirmed => "confirmed",
            Confidence::Probable => "probable",
            Confidence::Informational => "informational",
            Confidence::LikelyFalsePositive => "likely_false_positive",
        }
    }
}

/// A single Layer-A Unicode watermark finding inside a text string.
///
/// Produced by [`crate::unicode::inspect_text`] and collected into a
/// [`TextInspectReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharHit {
    /// The Unicode scalar value of the suspicious character.
    pub codepoint: u32,

    /// The suspicious character itself (may not render visibly).
    pub character: String,

    /// A human-readable label, e.g. `"U+200B ZERO WIDTH SPACE (Cf)"`.
    pub label: String,

    /// Number of occurrences in the inspected text.
    pub count: usize,

    /// Classification of the watermark kind (e.g. `ZwjFamily`, `Bidi`).
    pub kind: WatermarkKind,

    /// Detection confidence for this character class.
    pub confidence: Confidence,

    /// Up to 10 byte offsets in the original string where the character
    /// appears (UTF-8 char boundary offsets).
    pub sample_offsets: Vec<usize>,
}

/// A single file-metadata or container-level watermark finding.
///
/// Produced by [`crate::image_meta`] and [`crate::container_meta`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFinding {
    /// Human-readable description of the finding, e.g.
    /// `"PNG chunk C2PA (C2PA provenance manifest)"`.
    pub description: String,

    /// How confident the detection is.
    pub confidence: Confidence,

    /// The specific watermark kind, if it can be classified.
    pub kind: Option<WatermarkKind>,
}

/// Full Layer-A inspection result for a text string.
///
/// Returned by [`crate::unicode::inspect_text`].
///
/// # Complexity
/// - Time: O(n) where n is the number of Unicode scalar values in the text.
/// - Space: O(k) where k is the number of distinct suspicious codepoints found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInspectReport {
    /// Number of Unicode scalar values in the inspected text.
    pub length: usize,

    /// Total count of suspicious characters found (sum of all hit counts).
    pub suspicious_total: usize,

    /// Per-codepoint findings, sorted by descending occurrence count.
    pub hits: Vec<CharHit>,

    /// Informational notes about the inspection scope and limitations.
    pub notes: Vec<String>,
}

/// The image format detected or supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    /// Portable Network Graphics (PNG).
    Png,
    /// JPEG / JFIF.
    Jpeg,
    /// WebP (RIFF container).
    Webp,
    /// Scalable Vector Graphics (SVG / XML text).
    Svg,
}

impl ImageFormat {
    /// Returns the MIME type for the format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Webp => "image/webp",
            ImageFormat::Svg => "image/svg+xml",
        }
    }
}

/// Full metadata inspection result for an image.
///
/// Returned by [`crate::image_meta::inspect_image`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInspectReport {
    /// Detected image format.
    pub format: ImageFormat,

    /// All metadata findings discovered in the image.
    pub findings: Vec<MetaFinding>,
}

/// Caller-supplied or auto-detected media format hint.
///
/// Pass to [`crate::cleaner::clean`] and [`crate::cleaner::inspect`]. When
/// `None`, the functions auto-detect the format from magic bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaHint {
    /// Plain UTF-8 text (no file container).
    Text,
    /// PNG image.
    Png,
    /// JPEG image.
    Jpeg,
    /// WebP image.
    Webp,
    /// SVG document.
    Svg,
    /// PDF document.
    Pdf,
    /// DOCX document (Office Open XML ZIP container).
    Docx,
    /// ODT document (ODF ZIP container).
    Odt,
    /// HTML document.
    Html,
    /// Markdown document.
    Markdown,
}

/// Statistics collected during a cleaning pass.
///
/// Counts are keyed by [`WatermarkKind`] string for easy serialisation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanStats {
    /// Number of characters / bytes removed outright.
    pub removed_count: usize,

    /// Number of characters / bytes replaced with a canonical equivalent.
    pub replaced_count: usize,

    /// Number of metadata chunks or sections stripped from a file.
    pub metadata_chunks_removed: usize,

    /// Human-readable summary lines, one per operation performed.
    pub summary: Vec<String>,
}

/// Output of a successful [`crate::cleaner::clean`] call.
#[derive(Debug, Clone)]
pub struct CleanOutput {
    /// The cleaned bytes (text re-encoded as UTF-8, or cleaned binary).
    pub bytes: Vec<u8>,

    /// Statistics about what was removed or replaced.
    pub stats: CleanStats,

    /// The media format that was cleaned.
    pub format: MediaHint,
}

/// Output of a successful [`crate::cleaner::inspect`] call.
#[derive(Debug, Clone)]
pub struct InspectOutput {
    /// Text-level findings (populated for text and document inputs).
    pub text_report: Option<TextInspectReport>,

    /// Image-metadata findings (populated for image inputs).
    pub image_report: Option<ImageInspectReport>,

    /// Container-level metadata findings (populated for file inputs).
    pub meta_findings: Vec<MetaFinding>,

    /// The media format that was inspected.
    pub format: MediaHint,
}
