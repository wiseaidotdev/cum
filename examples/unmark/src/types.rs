// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Application-level types for the Unmark front-end.

use serde::{Deserialize, Serialize};

/// Language selection for the stochastic synonym layer.
///
/// Mirrors the `LanguageHint` enum in `cum_rs::stochastic` but is defined
/// here to avoid a WASM dependency on the internal crate enum serialisation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AppLanguage {
    #[default]
    Auto,
    English,
    Spanish,
    French,
    German,
    Arabic,
}

impl AppLanguage {
    /// Returns the display label shown in the language dropdown.
    pub fn label(&self) -> &'static str {
        match self {
            AppLanguage::Auto => "Auto-detect",
            AppLanguage::English => "English",
            AppLanguage::Spanish => "Español",
            AppLanguage::French => "Français",
            AppLanguage::German => "Deutsch",
            AppLanguage::Arabic => "العربية",
        }
    }

    /// Returns the `value` attribute string for `<option>` elements.
    pub fn value(&self) -> &'static str {
        match self {
            AppLanguage::Auto => "auto",
            AppLanguage::English => "en",
            AppLanguage::Spanish => "es",
            AppLanguage::French => "fr",
            AppLanguage::German => "de",
            AppLanguage::Arabic => "ar",
        }
    }

    /// Parses from the value attribute string.
    pub fn from_value(s: &str) -> Self {
        match s {
            "en" => AppLanguage::English,
            "es" => AppLanguage::Spanish,
            "fr" => AppLanguage::French,
            "de" => AppLanguage::German,
            "ar" => AppLanguage::Arabic,
            _ => AppLanguage::Auto,
        }
    }
}

/// Configuration for the stochastic synonym-replacement layer.
///
/// Carried as Yew state in the top-level [`App`] component and forwarded to
/// the `ControlsPanel` and the enhancement call in `run_enhance_text`.
#[derive(Debug, Clone, PartialEq)]
pub struct StochasticConfig {
    /// Whether synonym replacement is currently enabled.
    pub enabled: bool,
    /// Per-word substitution probability in `[0.0, 1.0]`.
    ///
    /// Stored as a percentage integer (1-100) by the slider and divided at
    /// call time to keep the UI arithmetic simple.
    pub probability_pct: u32,
    /// Language to use for the curated synonym table.
    pub language: AppLanguage,
    /// Whether to normalise curly quotes, em-dashes and ellipses (Layer A).
    pub normalize_punctuation: bool,
}

/// The active input mode: determines the left-panel UI rendered.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// Plain text entered in the textarea.
    Text,
    /// Binary file (image or document) loaded via file picker or drag-and-drop.
    File,
}

/// The kind of media the user submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaKind {
    /// Plain text.
    Text,
    /// An image (`image/png`, `image/jpeg`, `image/webp`, `image/svg+xml`).
    Image(String),
    /// A document (`application/pdf`, `.docx`, etc.).
    Document(String),
}

impl MediaKind {
    /// Returns the CSS badge class for the media kind indicator.
    pub fn badge_class(&self) -> &'static str {
        match self {
            MediaKind::Text => "um-badge um-badge-text",
            MediaKind::Image(_) => "um-badge um-badge-image",
            MediaKind::Document(_) => "um-badge um-badge-doc",
        }
    }

    /// Returns the icon class for the media kind indicator.
    pub fn icon(&self) -> &'static str {
        match self {
            MediaKind::Text => "fa-solid fa-font",
            MediaKind::Image(_) => "fa-solid fa-image",
            MediaKind::Document(_) => "fa-solid fa-file-lines",
        }
    }

    /// Returns the label for the media kind badge.
    pub fn label(&self) -> &str {
        match self {
            MediaKind::Text => "Text",
            MediaKind::Image(mime) => mime.as_str(),
            MediaKind::Document(name) => name.as_str(),
        }
    }
}

/// Statistics returned by the WASM cleaner.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CleanStats {
    pub removed_count: usize,
    pub replaced_count: usize,
    pub metadata_chunks_removed: usize,
    pub summary: Vec<String>,
}

impl CleanStats {
    /// Returns the total number of watermarks removed or replaced.
    pub fn total_marks(&self) -> usize {
        self.removed_count + self.replaced_count + self.metadata_chunks_removed
    }
}

/// The result of a cleaning operation ready for display.
#[derive(Debug, Clone, PartialEq)]
pub struct CleanResult {
    /// Cleaned bytes (text or binary).
    pub bytes: Vec<u8>,
    /// Statistics about what was removed.
    pub stats: CleanStats,
    /// The media kind of the output.
    pub kind: MediaKind,
    /// If `kind` is `Image`, the data-URL for `<img src>`.
    pub image_data_url: Option<String>,
}
