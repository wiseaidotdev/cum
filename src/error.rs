// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Error Types
//!
//! This module defines all error variants produced by `cum-rs`. Every public
//! function that can fail returns [`Result<T>`], which is an alias for
//! `std::result::Result<T, CumError>`.
//!
//! ## Error Variants
//!
//! | Variant | Condition |
//! |---------|-----------|
//! | [`CumError::Io`] | Underlying I/O failure (file read/write). |
//! | [`CumError::UnsupportedFormat`] | The byte stream matches no supported media type. |
//! | [`CumError::ParseError`] | Malformed chunk data or invalid encoding. |
//! | [`CumError::InputTooLarge`] | Input exceeds the configured byte-length cap. |
//! | [`CumError::BinaryInput`] | Text-only API was handed binary data. |

use thiserror::Error;

/// The primary error type returned by all fallible `cum-rs` functions.
///
/// Implements [`std::error::Error`] via the `thiserror` derive macro so it
/// composes cleanly with `?` and `anyhow`.
///
/// # Example
/// ```
/// use cum_rs::error::{CumError, Result};
///
/// fn may_fail(input: &[u8]) -> Result<()> {
///     if input.is_empty() {
///         return Err(CumError::ParseError("empty input".into()));
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Error)]
pub enum CumError {
    /// An I/O error from the operating system or the `std::io` layer.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The byte stream does not match any format the crate recognises.
    ///
    /// The contained string names the attempted format (e.g. `"PNG"`,
    /// `"DOCX"`).
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// The byte stream is structurally malformed (e.g. truncated chunk,
    /// invalid UTF-8 where UTF-8 is required, bad ZIP archive).
    ///
    /// The contained string provides a human-readable description of the
    /// parse failure.
    #[error("parse error: {0}")]
    ParseError(String),

    /// The caller supplied more bytes than the crate's safety cap allows.
    ///
    /// Whole-file in-memory processing means unlimited input is a memory-DoS
    /// vector; this error surfaces when the cap is hit so callers can retry
    /// with a higher limit or reject the input.
    #[error("input too large: limit {limit} bytes, got {actual} bytes")]
    InputTooLarge {
        /// Maximum number of bytes the operation accepts.
        limit: usize,
        /// Actual number of bytes in the input.
        actual: usize,
    },

    /// A text-only API (e.g. [`crate::unicode::clean_text`]) received data
    /// that heuristics classify as binary (magic bytes, NUL, high control
    /// density).
    ///
    /// The contained string describes what kind of binary the data looks like
    /// (e.g. `"a PNG image"`).
    #[error("binary input: {0}")]
    BinaryInput(String),

    /// An error occurred during ZIP archive processing (DOCX/ODT containers).
    #[error("zip error: {0}")]
    Zip(String),
}

/// A `Result` type alias that pins the error to [`CumError`].
///
/// All fallible functions in `cum-rs` return this type.
pub type Result<T> = std::result::Result<T, CumError>;
