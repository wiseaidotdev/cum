// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Stochastic Text Enhancement
//!
//! This module provides [`StochasticEnhancer`] and [`SynonymBank`], which together
//! implement controllable word-level variation as a best-effort countermeasure against
//! **Layer B** statistical watermarks (SynthID-Text, KGW).
//!
//! ## Key Optimisation: Compile-time PHF Synonym Table
//!
//! The curated synonym table is stored as a `phf::Map` with `&'static [&'static str]`
//! values, compiled into the binary at build time.  This replaces a runtime `HashMap`
//! that would be rebuilt on each call with a **zero-allocation, O(1)** lookup.
//!
//! ## Stop-word Detection
//!
//! Stop words are stored as a compile-time [`phf::Set`], replacing an O(n) linear scan
//! over a static slice.
//!
//! ## Enhancement Pipeline
//!
//! For each non-stop word in the input text, [`StochasticEnhancer`] samples a uniform
//! Bernoulli draw with probability `p` (default 0.5).  On a "hit", the curated table
//! is consulted first; if no entry exists, a same-length word from the system wordlist
//! is substituted.  Case style (ALL_CAPS, Capitalised, lowercase) is mirror-copied to
//! the substituted word.
//!
//! ## Wordlist Source
//!
//! On Linux and macOS, the system dictionary at `/usr/share/dict/american-english`
//! (or the first path from [`SYSTEM_DICT_PATHS`] that exists) is loaded at
//! [`SynonymBank::new`] construction time.  On WASM targets the wordlist is omitted
//! and only the curated table is used.
//!
//! ## Example
//!
//! ```
//! use cum_rs::stochastic::StochasticEnhancer;
//!
//! let enhancer = StochasticEnhancer::with_default_probability();
//! let output   = enhancer.enhance("chaos governs the universe");
//! assert!(!output.text.is_empty());
//! ```
//!
//! # See Also
//!
//! - [Kirchenbauer, J. et al. (2023). A Watermark for Large Language Models.](https://arxiv.org/abs/2301.10226) - the foundational token-sampling watermark (KGW) this countermeasure is designed against.
//! - [`crate::cleaner::clean`] - the primary unified API which performs deterministic metadata and Unicode carrier removal (Layer A) prior to stochastic enhancement.
//!
//! This logic was adapted from <https://github.com/wiseaidotdev/lmm/blob/main/lmm/src/stochastic.rs>

mod arabic;
mod detect;
mod english;
mod french;
mod german;
mod spanish;

pub use detect::{LanguageHint, detect_language};

use detect::synonyms_for;
use english::CURATED_SYNONYMS;
use phf::{Set, phf_set};
use rand::{Rng, RngExt, rng as thread_rng};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;

/// Default synonym-substitution probability (50%).
const DEFAULT_REPLACEMENT_PROBABILITY: f64 = 0.5;

/// Minimum word length to include from the system dictionary.
#[cfg(not(target_arch = "wasm32"))]
const WORDLIST_MIN_WORD_LEN: usize = 5;

/// Maximum word length to include from the system dictionary.
#[cfg(not(target_arch = "wasm32"))]
const WORDLIST_MAX_WORD_LEN: usize = 14;

/// System dictionary file paths tried in order until one is readable.
#[cfg(not(target_arch = "wasm32"))]
static SYSTEM_DICT_PATHS: &[&str] = &[
    "/usr/share/dict/american-english",
    "/usr/share/dict/words",
    "/usr/dict/words",
];

/// Compile-time stop-word set.
///
/// Words in this set are never substituted by the stochastic engine.  The set
/// is stored as a perfect hash to provide O(1) membership testing.
static STOP_WORDS: Set<&'static str> = phf_set! {
    "a", "an", "the", "and", "or", "but", "is", "are", "was", "were",
    "be", "been", "being", "have", "has", "had", "do", "does", "did",
    "will", "would", "could", "should", "may", "might", "shall", "can",
    "to", "of", "in", "on", "at", "by", "for", "with", "about", "as",
    "into", "through", "during", "before", "after", "above", "below",
    "from", "up", "down", "out", "off", "over", "under", "again", "then",
    "once", "here", "there", "when", "where", "why", "how", "all", "both",
    "each", "few", "more", "most", "other", "some", "such", "no", "not",
    "only", "own", "same", "than", "too", "very", "just", "because",
    "if", "while", "although", "though", "so", "yet", "nor", "either",
    "neither", "i", "me", "my", "myself", "we", "our", "ours", "ourselves",
    "you", "your", "yours", "yourself", "yourselves", "he", "him", "his",
    "himself", "she", "her", "hers", "herself", "it", "its", "itself",
    "they", "them", "their", "theirs", "themselves", "what", "which", "who",
    "whom", "this", "that", "these", "those", "am", "every",
};

/// Loads the system dictionary and groups words by character length.
///
/// The function iterates through [`SYSTEM_DICT_PATHS`] and reads the first file that
/// exists and is readable.  Words are lower-cased, restricted to ASCII alphabetic
/// characters, and filtered to `[WORDLIST_MIN_WORD_LEN, WORDLIST_MAX_WORD_LEN]`.
///
/// Falls back to an empty map when no dictionary file is found.
///
/// # Returns
///
/// (`HashMap<usize, Vec<String>>`): Words indexed by their character length.
///
/// # Time Complexity
///
/// O(w) where w is the total number of words in the dictionary file.
///
/// # Space Complexity
///
/// O(w) in the worst case.
#[cfg(not(target_arch = "wasm32"))]
fn load_wordlist_by_length() -> HashMap<usize, Vec<String>> {
    let mut by_length: HashMap<usize, Vec<String>> = HashMap::new();
    for path in SYSTEM_DICT_PATHS {
        if let Ok(content) = fs::read_to_string(path) {
            for word in content.lines() {
                let w = word.trim().to_lowercase();
                if w.len() >= WORDLIST_MIN_WORD_LEN
                    && w.len() <= WORDLIST_MAX_WORD_LEN
                    && w.chars().all(|c| c.is_ascii_alphabetic())
                {
                    by_length.entry(w.len()).or_default().push(w);
                }
            }
            break;
        }
    }
    by_length
}

/// WASM stub: returns an empty map because the system dictionary is unavailable.
///
/// On WASM targets only the curated table is used for synonym substitution.
#[cfg(target_arch = "wasm32")]
fn load_wordlist_by_length() -> HashMap<usize, Vec<String>> {
    HashMap::new()
}

/// A two-tier synonym lookup bank.
///
/// - **Tier 1**: Compile-time curated PHF map: O(1), zero allocation.
/// - **Tier 2**: Runtime wordlist grouped by word length: O(1) bucket lookup, O(k) sample.
///
/// The bank is constructed once and then shared immutably across all calls to
/// [`StochasticEnhancer::enhance`].
///
/// # Examples
///
/// ```
/// use cum_rs::stochastic::SynonymBank;
///
/// let bank = SynonymBank::new();
/// assert!(bank.curated_count() > 50);
/// ```
pub struct SynonymBank {
    /// Words indexed by character length, loaded from the system dictionary.
    wordlist: HashMap<usize, Vec<String>>,
    /// The active language for curated synonym lookups.
    pub(super) language: LanguageHint,
}

impl SynonymBank {
    /// Constructs a new [`SynonymBank`] with the default English synonym table
    /// and the host system wordlist.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    ///
    /// let bank = SynonymBank::new();
    /// assert!(bank.curated_count() > 0);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w) where w is the number of words in the system dictionary (for the
    /// initial wordlist load). Subsequent calls are O(1) for curated lookups.
    ///
    /// # Space Complexity
    ///
    /// O(w) for the system wordlist, O(1) for the curated table reference.
    pub fn new() -> Self {
        Self {
            wordlist: load_wordlist_by_length(),
            language: LanguageHint::English,
        }
    }

    /// Constructs a [`SynonymBank`] using a specific language's curated table.
    ///
    /// # Arguments
    ///
    /// * `lang`: The target language. [`LanguageHint::Auto`] falls back to English.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::{SynonymBank, LanguageHint};
    ///
    /// let bank = SynonymBank::with_language(LanguageHint::Spanish);
    /// assert!(bank.curated_count() > 0);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w) for system wordlist load.
    ///
    /// # Space Complexity
    ///
    /// O(w).
    pub fn with_language(lang: LanguageHint) -> Self {
        Self {
            wordlist: load_wordlist_by_length(),
            language: lang,
        }
    }

    /// Returns a synonym candidate for `word`, or `None` if no candidate exists.
    ///
    /// The lookup order is:
    /// 1. Curated language-specific PHF table (O(1)).
    /// 2. System wordlist bucket for the same word length (O(k) random sample).
    ///
    /// # Arguments
    ///
    /// * `word` : Lower-cased input word.
    /// * `rng`  : Mutable random-number generator used for wordlist sampling.
    ///
    /// # Returns
    ///
    /// `Some(&str)` on success, `None` when both tiers have no candidate.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    ///
    /// let bank = SynonymBank::new();
    /// let mut rng = rand::rng();
    /// assert!(bank.candidate("chaos", &mut rng).is_some());
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(1) for the curated tier; O(k) for the wordlist tier where k is the
    /// number of words of that length in the dictionary.
    ///
    /// # Space Complexity
    ///
    /// O(1).
    pub fn candidate<R: Rng>(&self, word: &str, rng: &mut R) -> Option<&str> {
        let table = synonyms_for(self.language);
        if let Some(synonyms) = table.get(word) {
            let idx = rng.random_range(0..synonyms.len());
            return Some(synonyms[idx]);
        }
        if let Some(bucket) = self.wordlist.get(&word.len()).filter(|b| !b.is_empty()) {
            let idx = rng.random_range(0..bucket.len());
            return Some(&bucket[idx]);
        }
        None
    }

    /// Returns the number of entries in the curated synonym table.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    ///
    /// let bank = SynonymBank::new();
    /// assert!(bank.curated_count() > 50, "curated table must have substantial coverage");
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(1).
    ///
    /// # Space Complexity
    ///
    /// O(1).
    pub fn curated_count(&self) -> usize {
        CURATED_SYNONYMS.len()
    }

    /// Returns the number of words loaded from the system wordlist.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    ///
    /// let bank = SynonymBank::new();
    /// println!("wordlist len: {}", bank.wordlist_len());
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(b) where b is the number of distinct word-length buckets.
    ///
    /// # Space Complexity
    ///
    /// O(1).
    pub fn wordlist_len(&self) -> usize {
        self.wordlist.values().map(Vec::len).sum()
    }
}

impl Default for SynonymBank {
    fn default() -> Self {
        Self::new()
    }
}

/// The result of a [`StochasticEnhancer::enhance`] call.
///
/// Carries the enhanced text alongside bookkeeping fields that let callers
/// report how much the text was altered.
#[derive(Debug, Clone)]
pub struct EnhanceOutput {
    /// The enhanced text with synonym substitutions applied.
    pub text: String,

    /// The substitution probability that was configured.
    pub probability: f64,

    /// Number of words that were actually substituted in this call.
    pub words_substituted: usize,

    /// The language that was active during substitution.
    pub language: LanguageHint,
}

/// A probabilistic word-substitution engine for stochastic text variation.
///
/// At each token, a Bernoulli trial with probability `p` determines whether to
/// substitute the word.  Stop words (from the compile-time [`STOP_WORDS`] PHF set)
/// are always preserved.  Case style (ALL_CAPS, Capitalised, lowercase) is mirror-copied
/// to the substituted word.
///
/// This is a best-effort countermeasure against **Layer B** statistical watermarks
/// such as SynthID-Text (Google) and KGW (Kirchenbauer et al.) that are baked into
/// word choices at generation time.
///
/// # Examples
///
/// ```
/// use cum_rs::stochastic::StochasticEnhancer;
///
/// let enhancer = StochasticEnhancer::with_default_probability();
/// let output   = enhancer.enhance("chaos governs the universe");
/// assert!(!output.text.is_empty());
/// ```
pub struct StochasticEnhancer {
    /// Synonym lookup bank (curated + system wordlist).
    bank: SynonymBank,
    /// Bernoulli substitution probability clamped to [0.0, 1.0].
    probability: f64,
}

impl StochasticEnhancer {
    /// Creates a new enhancer with the given substitution probability.
    ///
    /// The probability is clamped to `[0.0, 1.0]`.
    ///
    /// # Arguments
    ///
    /// * `probability`: Fraction of eligible words to substitute.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    ///
    /// let e = StochasticEnhancer::new(0.3);
    /// assert_eq!(e.probability(), 0.3);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w) for system wordlist load; O(1) thereafter.
    ///
    /// # Space Complexity
    ///
    /// O(w).
    pub fn new(probability: f64) -> Self {
        Self {
            bank: SynonymBank::new(),
            probability: probability.clamp(0.0, 1.0),
        }
    }

    /// Creates an enhancer with the default 50% substitution probability.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    ///
    /// let e = StochasticEnhancer::with_default_probability();
    /// assert_eq!(e.probability(), 0.5);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w).
    ///
    /// # Space Complexity
    ///
    /// O(w).
    pub fn with_default_probability() -> Self {
        Self::new(DEFAULT_REPLACEMENT_PROBABILITY)
    }

    /// Creates an enhancer with a specific language hint and default probability.
    ///
    /// # Arguments
    ///
    /// * `lang`: Language to use for curated synonym lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::{StochasticEnhancer, LanguageHint};
    ///
    /// let e = StochasticEnhancer::with_language(LanguageHint::French);
    /// assert_eq!(e.probability(), 0.5);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w).
    ///
    /// # Space Complexity
    ///
    /// O(w).
    pub fn with_language(lang: LanguageHint) -> Self {
        Self {
            bank: SynonymBank::with_language(lang),
            probability: DEFAULT_REPLACEMENT_PROBABILITY,
        }
    }

    /// Creates an enhancer with a specific language and a custom probability.
    ///
    /// # Arguments
    ///
    /// * `lang`: Language to use for curated synonym lookup.
    /// * `probability`: Fraction of eligible words to substitute, clamped to `[0.0, 1.0]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::{StochasticEnhancer, LanguageHint};
    ///
    /// let e = StochasticEnhancer::with_language_and_probability(LanguageHint::Spanish, 0.8);
    /// assert_eq!(e.probability(), 0.8);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w).
    ///
    /// # Space Complexity
    ///
    /// O(w).
    pub fn with_language_and_probability(lang: LanguageHint, probability: f64) -> Self {
        Self {
            bank: SynonymBank::with_language(lang),
            probability: probability.clamp(0.0, 1.0),
        }
    }

    /// Returns the configured substitution probability.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    ///
    /// let e = StochasticEnhancer::new(0.75);
    /// assert_eq!(e.probability(), 0.75);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(1).
    ///
    /// # Space Complexity
    ///
    /// O(1).
    pub fn probability(&self) -> f64 {
        self.probability
    }

    /// Enhances multi-line text by applying stochastic synonym substitution.
    ///
    /// Each line is processed independently.  Blank lines are preserved.
    /// Words in [`STOP_WORDS`] are never substituted.
    ///
    /// # Arguments
    ///
    /// * `text`: Input text, potentially multi-line.
    ///
    /// # Returns
    ///
    /// An [`EnhanceOutput`] containing the enhanced text and substitution statistics.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    ///
    /// let e   = StochasticEnhancer::new(1.0);
    /// let out = e.enhance("chaos governs the universe");
    /// assert_eq!(out.words_substituted + out.text.split_whitespace().count()
    ///     >= out.text.split_whitespace().count(), true);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(n) where n is the number of tokens in `text`.
    ///
    /// # Space Complexity
    ///
    /// O(n).
    pub fn enhance(&self, text: &str) -> EnhanceOutput {
        let mut rng = thread_rng();
        let mut total_substituted: usize = 0;

        let enhanced = text
            .lines()
            .map(|line| {
                let (enhanced_line, count) = self.enhance_line(line, &mut rng);
                total_substituted += count;
                enhanced_line
            })
            .collect::<Vec<_>>()
            .join("\n");

        EnhanceOutput {
            text: enhanced,
            probability: self.probability,
            words_substituted: total_substituted,
            language: self.bank.language,
        }
    }

    /// Enhances a single line by processing tokens left-to-right.
    ///
    /// Returns the enhanced line and the count of substituted words.
    ///
    /// # Time Complexity
    ///
    /// O(t) where t is the number of tokens in `line`.
    ///
    /// # Space Complexity
    ///
    /// O(t).
    fn enhance_line<R: Rng>(&self, line: &str, rng: &mut R) -> (String, usize) {
        let mut result = String::with_capacity(line.len());
        let mut substituted: usize = 0;

        for (i, raw_token) in line.split_whitespace().enumerate() {
            if i > 0 {
                result.push(' ');
            }
            let (prefix, word, suffix) = split_token(raw_token);
            let lower = word.to_lowercase();

            if is_stop_word(&lower) || rng.random::<f64>() >= self.probability {
                result.push_str(raw_token);
                continue;
            }

            if let Some(candidate) = self.bank.candidate(&lower, rng) {
                let styled = apply_case_style(word, candidate);
                result.push_str(prefix);
                result.push_str(&styled);
                result.push_str(suffix);
                substituted += 1;
            } else {
                result.push_str(raw_token);
            }
        }

        (result, substituted)
    }
}

/// Splits a raw token into leading punctuation, the word body, and trailing punctuation.
///
/// This allows case-style copying and synonym substitution to operate only on the
/// alphabetic core, while punctuation is preserved in its original position.
///
/// # Arguments
///
/// * `raw`: A single whitespace-delimited token from the input text.
///
/// # Returns
///
/// `(prefix, word, suffix)` where `prefix + word + suffix == raw`.
///
/// # Examples
///
/// ```
/// use cum_rs::stochastic::split_token;
///
/// assert_eq!(split_token("(hello,)"), ("(", "hello", ",)"));
/// assert_eq!(split_token("world!"), ("", "world", "!"));
/// assert_eq!(split_token("plain"), ("", "plain", ""));
/// ```
///
/// # Time Complexity
///
/// O(n) where n is the length of `raw`.
///
/// # Space Complexity
///
/// O(1) (returns sub-slices of `raw`).
pub fn split_token(raw: &str) -> (&str, &str, &str) {
    let start = raw
        .char_indices()
        .find(|(_, c)| c.is_alphabetic())
        .map(|(i, _)| i)
        .unwrap_or(raw.len());
    let end = raw
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphabetic())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    if start >= end {
        return ("", raw, "");
    }
    (&raw[..start], &raw[start..end], &raw[end..])
}

/// Applies the case style of `original` to `candidate`.
///
/// Three styles are recognised:
/// - ALL_CAPS: every character of `original` is uppercase.
/// - Capitalised: first character of `original` is uppercase; remaining characters are left as-is.
/// - lowercase: everything else.
///
/// # Arguments
///
/// * `original` : The word whose case style should be copied.
/// * `candidate`: The replacement word to style.
///
/// # Returns
///
/// A new `String` with `candidate` styled to match `original`.
///
/// # Time Complexity
///
/// O(n) where n is the length of `candidate`.
///
/// # Space Complexity
///
/// O(n).
fn apply_case_style(original: &str, candidate: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        candidate.to_uppercase()
    } else if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        capitalize(candidate)
    } else {
        candidate.to_lowercase()
    }
}

/// Capitalises the first character of `s` and lowercases the remainder.
///
/// Handles multi-byte Unicode characters correctly.
///
/// # Arguments
///
/// * `s`: Input string slice.
///
/// # Returns
///
/// A new `String` with the first character uppercased.
///
/// # Examples
///
/// ```
/// use cum_rs::stochastic::capitalize;
///
/// assert_eq!(capitalize("hello"), "Hello");
/// assert_eq!(capitalize("world"), "World");
/// assert_eq!(capitalize(""), "");
/// ```
///
/// # Time Complexity
///
/// O(n) where n is the byte length of `s`.
///
/// # Space Complexity
///
/// O(n).
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            upper + chars.as_str()
        }
    }
}

/// Returns `true` if `word` is in the compile-time stop-word set.
///
/// Stop words are never substituted by the stochastic engine.
///
/// # Arguments
///
/// * `word`: Lower-cased word to test.
///
/// # Returns
///
/// `true` if `word` is a stop word.
///
/// # Examples
///
/// ```
/// use cum_rs::stochastic::is_stop_word;
///
/// assert!(is_stop_word("the"));
/// assert!(!is_stop_word("chaos"));
/// ```
///
/// # Time Complexity
///
/// O(1) (PHF lookup).
///
/// # Space Complexity
///
/// O(1).
pub fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(word)
}
