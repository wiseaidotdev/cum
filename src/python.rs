// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Python Bindings
//!
//! PyO3-based extension module exposing `cum_rs` to Python 3.8+.
//!
//! ## Usage (after `pip install cum-rs`)
//!
//! ```python
//! import cum_rs
//!
//! result = cum_rs.clean_text("Hello\u200b world\ufeff!")
//! print(result.cleaned)           # "Hello world!"
//! print(result.removed_count)     # 2
//!
//! report = cum_rs.inspect_text("Hello\u200b world!")
//! for hit in report.hits:
//!     print(hit.label, hit.count)
//! ```

use crate::cleaner::clean;
use crate::unicode::{CleanOpts, InspectOpts, clean_text, inspect_text};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Python class wrapping the result of [`clean_text`].
#[pyclass(name = "CleanTextResult")]
pub struct PyCleanTextResult {
    /// The cleaned text string with all watermark carriers removed.
    #[pyo3(get)]
    pub cleaned: String,
    /// Number of characters removed outright (invisible carriers).
    #[pyo3(get)]
    pub removed_count: usize,
    /// Number of characters replaced with canonical equivalents (e.g. space
    /// homoglyphs → ASCII space).
    #[pyo3(get)]
    pub replaced_count: usize,
    /// Human-readable summary lines, one per operation performed.
    #[pyo3(get)]
    pub summary: Vec<String>,
}

/// Python class wrapping a single Layer-A character finding.
#[pyclass(name = "CharHit", skip_from_py_object)]
#[derive(Clone)]
pub struct PyCharHit {
    /// Unicode scalar value of the suspicious codepoint.
    #[pyo3(get)]
    pub codepoint: u32,
    /// The suspicious character itself.
    #[pyo3(get)]
    pub character: String,
    /// Human-readable label (e.g. `"U+200B ZERO WIDTH SPACE (Cf)"`).
    #[pyo3(get)]
    pub label: String,
    /// Number of occurrences in the inspected text.
    #[pyo3(get)]
    pub count: usize,
    /// Watermark kind string (e.g. `"zwj_family"`).
    #[pyo3(get)]
    pub kind: String,
    /// Detection confidence string (e.g. `"probable"`).
    #[pyo3(get)]
    pub confidence: String,
    /// Up to 10 character offsets where the codepoint appears.
    #[pyo3(get)]
    pub sample_offsets: Vec<usize>,
}

/// Python class wrapping a full Layer-A text inspection report.
#[pyclass(name = "TextInspectReport")]
pub struct PyTextInspectReport {
    /// Number of Unicode scalar values in the inspected text.
    #[pyo3(get)]
    pub length: usize,
    /// Total count of suspicious characters.
    #[pyo3(get)]
    pub suspicious_total: usize,
    /// Per-codepoint findings.
    #[pyo3(get)]
    pub hits: Vec<PyCharHit>,
    /// Informational notes about scope and limitations.
    #[pyo3(get)]
    pub notes: Vec<String>,
}

/// Removes all Layer-A Unicode watermark carriers from a Python string.
///
/// # Arguments
/// * `text`: the text to clean.
///
/// # Returns
/// A [`PyCleanTextResult`] with the cleaned string and statistics.
///
/// # Raises
/// `RuntimeError` if the input exceeds 256 MiB.
#[pyfunction(name = "clean_text")]
pub fn clean_text_py(text: &str) -> PyResult<PyCleanTextResult> {
    let opts = CleanOpts::safe();
    let (cleaned, stats) = clean_text(text, &opts)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyCleanTextResult {
        cleaned,
        removed_count: stats.removed_count,
        replaced_count: stats.replaced_count,
        summary: stats.summary,
    })
}

/// Inspects a Python string for Layer-A Unicode watermark carriers.
///
/// # Arguments
/// * `text`: the text to inspect.
///
/// # Returns
/// A [`PyTextInspectReport`] with per-codepoint findings.
///
/// # Raises
/// `RuntimeError` if the input exceeds 256 MiB.
#[pyfunction(name = "inspect_text")]
pub fn inspect_text_py(text: &str) -> PyResult<PyTextInspectReport> {
    let opts = InspectOpts::default();
    let report = inspect_text(text, &opts)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let hits = report
        .hits
        .into_iter()
        .map(|h| PyCharHit {
            codepoint: h.codepoint,
            character: h.character,
            label: h.label,
            count: h.count,
            kind: h.kind.as_str().to_string(),
            confidence: h.confidence.as_str().to_string(),
            sample_offsets: h.sample_offsets,
        })
        .collect();
    Ok(PyTextInspectReport {
        length: report.length,
        suspicious_total: report.suspicious_total,
        hits,
        notes: report.notes,
    })
}

/// Removes all detectable watermarks from raw bytes (image or document).
///
/// The format is auto-detected from magic bytes.
///
/// # Arguments
/// * `data`: the raw bytes as a Python `bytes` object.
///
/// # Returns
/// A Python `bytes` object with the cleaned content.
///
/// # Raises
/// `RuntimeError` on parse failure or unsupported format.
#[pyfunction(name = "clean_bytes")]
pub fn clean_bytes_py<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    let out =
        clean(data, None).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBytes::new(py, &out.bytes))
}

/// Registers all `cum_rs` Python functions and classes into the given module.
pub fn register_python_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(clean_text_py, m)?)?;
    m.add_function(wrap_pyfunction!(inspect_text_py, m)?)?;
    m.add_function(wrap_pyfunction!(clean_bytes_py, m)?)?;
    m.add_class::<PyCleanTextResult>()?;
    m.add_class::<PyTextInspectReport>()?;
    m.add_class::<PyCharHit>()?;
    Ok(())
}
