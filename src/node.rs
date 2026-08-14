// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Node.js Bindings
//!
//! napi-rs native add-on exposing `cum_rs` to Node.js.
//!
//! ## Usage (after `npm install cum-rs`)
//!
//! ```javascript
//! const { cleanText, inspectText, cleanBytes } = require('cum-rs');
//!
//! const result = cleanText("Hello\u200b world\ufeff!");
//! console.log(result.cleaned);       // "Hello world!"
//! console.log(result.removedCount);  // 2
//! ```

use crate::cleaner::clean;
use crate::unicode::{CleanOpts, InspectOpts, clean_text, inspect_text};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Result of a [`clean_text_node`] call.
#[napi(object)]
pub struct CleanTextResult {
    /// The cleaned text string with all watermark carriers removed.
    pub cleaned: String,
    /// Number of characters removed outright.
    pub removed_count: u32,
    /// Number of characters replaced with canonical equivalents.
    pub replaced_count: u32,
    /// Human-readable summary, one line per operation.
    pub summary: Vec<String>,
}

/// A single Layer-A character finding from [`inspect_text_node`].
#[napi(object)]
pub struct CharHit {
    /// Unicode scalar value of the suspicious codepoint.
    pub codepoint: u32,
    /// The suspicious character as a string.
    pub character: String,
    /// Human-readable label.
    pub label: String,
    /// Number of occurrences.
    pub count: u32,
    /// Watermark kind string (e.g. `"zwj_family"`).
    pub kind: String,
    /// Confidence string (e.g. `"probable"`).
    pub confidence: String,
    /// Up to 10 character offsets.
    pub sample_offsets: Vec<u32>,
}

/// Result of an [`inspect_text_node`] call.
#[napi(object)]
pub struct TextInspectReport {
    /// Number of Unicode scalar values in the inspected text.
    pub length: u32,
    /// Total count of suspicious characters.
    pub suspicious_total: u32,
    /// Per-codepoint findings.
    pub hits: Vec<CharHit>,
    /// Informational notes.
    pub notes: Vec<String>,
}

/// Removes all Layer-A Unicode watermark carriers from a string.
///
/// # Arguments
/// * `text`: the text to clean.
///
/// # Returns
/// A [`CleanTextResult`] with the cleaned string and statistics.
#[napi(js_name = "cleanText")]
pub fn clean_text_node(text: String) -> napi::Result<CleanTextResult> {
    let opts = CleanOpts::safe();
    let (cleaned, stats) =
        clean_text(&text, &opts).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(CleanTextResult {
        cleaned,
        removed_count: stats.removed_count as u32,
        replaced_count: stats.replaced_count as u32,
        summary: stats.summary,
    })
}

/// Inspects a string for Layer-A Unicode watermark carriers.
///
/// # Arguments
/// * `text`: the text to inspect.
///
/// # Returns
/// A [`TextInspectReport`] with per-codepoint findings.
#[napi(js_name = "inspectText")]
pub fn inspect_text_node(text: String) -> napi::Result<TextInspectReport> {
    let opts = InspectOpts::default();
    let report = inspect_text(&text, &opts).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    let hits = report
        .hits
        .into_iter()
        .map(|h| CharHit {
            codepoint: h.codepoint,
            character: h.character,
            label: h.label,
            count: h.count as u32,
            kind: h.kind.as_str().to_string(),
            confidence: h.confidence.as_str().to_string(),
            sample_offsets: h.sample_offsets.into_iter().map(|o| o as u32).collect(),
        })
        .collect();
    Ok(TextInspectReport {
        length: report.length as u32,
        suspicious_total: report.suspicious_total as u32,
        hits,
        notes: report.notes,
    })
}

/// Removes all detectable watermarks from a `Buffer` (image or document).
///
/// The format is auto-detected from magic bytes.
///
/// # Arguments
/// * `data`: the raw bytes as a Node.js `Buffer`.
///
/// # Returns
/// A `Buffer` containing the cleaned bytes.
#[napi(js_name = "cleanBytes")]
pub fn clean_bytes_node(data: Buffer) -> napi::Result<Buffer> {
    let bytes: &[u8] = &data;
    let out = clean(bytes, None).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Buffer::from(out.bytes))
}
