// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Language Detection and Dispatch
//!
//! Provides the [`LanguageHint`] enum and two functions:
//!
//! - [`detect_language`]: O(min(n, 512)) heuristic language detector.
//! - [`synonyms_for`]: O(1) dispatch to the correct static PHF synonym map.

use phf::Map;

use super::arabic::ARABIC_SYNONYMS;
use super::english::CURATED_SYNONYMS;
use super::french::FRENCH_SYNONYMS;
use super::german::GERMAN_SYNONYMS;
use super::spanish::SPANISH_SYNONYMS;

/// A hint that controls which curated synonym table is consulted during
/// stochastic text enhancement.
///
/// When set to [`Auto`](LanguageHint::Auto), the caller should first run
/// [`detect_language`] to obtain a concrete variant before constructing a
/// [`super::SynonymBank`].
///
/// # Examples
///
/// ```
/// use cum_rs::stochastic::LanguageHint;
/// assert_eq!(LanguageHint::Spanish.as_bcp47(), "es");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanguageHint {
    /// English (default).
    #[default]
    English,
    /// Spanish (Castilian).
    Spanish,
    /// Modern Standard Arabic.
    Arabic,
    /// French.
    French,
    /// German.
    German,
    /// Detect the language automatically from the input text.
    Auto,
}

impl LanguageHint {
    /// Returns the BCP-47 language tag for this hint.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::LanguageHint;
    /// assert_eq!(LanguageHint::Spanish.as_bcp47(), "es");
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(1).
    ///
    /// # Space Complexity
    ///
    /// O(1).
    pub fn as_bcp47(self) -> &'static str {
        match self {
            LanguageHint::English => "en",
            LanguageHint::Spanish => "es",
            LanguageHint::Arabic => "ar",
            LanguageHint::French => "fr",
            LanguageHint::German => "de",
            LanguageHint::Auto => "auto",
        }
    }
}

/// Detects the probable language of a text sample using character-range
/// heuristics and trigram frequency.
///
/// The detection checks for Arabic Unicode block presence first (unambiguous),
/// then checks characteristic Latin trigrams to distinguish Spanish, French,
/// and German from each other and from English.  Falls back to
/// [`LanguageHint::English`] when the evidence is inconclusive.
///
/// Only the first 512 code points of `text` are examined to keep the cost
/// proportional to a small constant rather than the full input.
///
/// # Arguments
///
/// * `text` - Input text sample.
///
/// # Returns
///
/// A [`LanguageHint`] best matching the dominant script / language.
///
/// # Time Complexity
///
/// O(min(n, 512)) where n is the number of code points in `text`.
///
/// # Space Complexity
///
/// O(1).
pub fn detect_language(text: &str) -> LanguageHint {
    let sample: String = text.chars().take(512).collect();

    let arabic_count = sample
        .chars()
        .filter(|&c| ('\u{0600}'..='\u{06FF}').contains(&c))
        .count();
    if arabic_count > 5 {
        return LanguageHint::Arabic;
    }

    let lower = sample.to_lowercase();

    let spanish_score: usize = [
        "ción", "que ", " de ", " en ", " la ", " el ", "ñ", "¿", "¡",
    ]
    .iter()
    .filter(|&&s| lower.contains(s))
    .count();
    let french_score: usize = [
        "tion", " de ", " le ", " la ", " un ", " des ", "œ", "ê", "â",
    ]
    .iter()
    .filter(|&&s| lower.contains(s))
    .count();
    let german_score: usize = [
        "sch ", " die ", " der ", " und ", " ist ", "ä", "ö", "ü", "ß",
    ]
    .iter()
    .filter(|&&s| lower.contains(s))
    .count();

    let max = spanish_score.max(french_score).max(german_score);
    if max < 2 {
        return LanguageHint::English;
    }
    if spanish_score == max {
        return LanguageHint::Spanish;
    }
    if french_score == max {
        return LanguageHint::French;
    }
    if german_score == max {
        return LanguageHint::German;
    }
    LanguageHint::English
}

/// Selects the appropriate curated synonym map for `lang`.
///
/// Returns a reference to one of the static `phf::Map` instances.
///
/// # Time Complexity
///
/// O(1).
///
/// # Space Complexity
///
/// O(1).
pub fn synonyms_for(lang: LanguageHint) -> &'static Map<&'static str, &'static [&'static str]> {
    match lang {
        LanguageHint::Spanish => &SPANISH_SYNONYMS,
        LanguageHint::French => &FRENCH_SYNONYMS,
        LanguageHint::German => &GERMAN_SYNONYMS,
        LanguageHint::Arabic => &ARABIC_SYNONYMS,
        _ => &CURATED_SYNONYMS,
    }
}
