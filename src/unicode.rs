// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Layer A: Invisible Unicode Watermark Detection and Removal
//!
//! This module provides deterministic, single-pass detection and removal of
//! Unicode characters that are commonly used as edit-based watermark carriers
//! in AI-generated text.
//!
//! ## Watermark Classes
//!
//! | Class | Examples | Action |
//! |-------|----------|--------|
//! | Invisible controls | ZWSP (U+200B), BOM (U+FEFF), WJ (U+2060) | Strip |
//! | Bidirectional format controls | LRE, RLE, LRO, RLO, PDF, LRI, RLI, FSI, PDI | Strip |
//! | Tag characters | U+E0001-U+E007F (except flag emoji runs) | Strip |
//! | Variation selectors | FE00-FE0F, E0100-E01EF (except after emoji/Mongolian) | Strip |
//! | Private-use characters | U+E000-F8FF, F0000-FFFFD, 100000-10FFFD | Strip |
//! | Space homoglyphs | En-Quad, Hair Space, Narrow NBSP, … → U+0020 | Replace |
//! | Cyrillic/fullwidth Latin confusables | Cyr А→A, FF21→A, … (aggressive mode) | Replace |
//!
//! ## Safe-keep Rules
//!
//! A small set of "invisible" characters are **orthographically load-bearing**
//! for specific scripts and are preserved by default:
//!
//! - ZWJ (U+200D) and VS-16 (U+FE0F) directly after an emoji base (emoji
//!   presentation and ZWJ sequences such as ❤️ and 👨‍👩‍👧).
//! - ZWNJ (U+200C) and ZWJ (U+200D) directly after a non-ASCII letter/mark
//!   (Persian word boundary, Devanagari half-form conjuncts).
//! - Tag characters U+E0020-U+E007F directly after a flag-sequence emoji base
//!   (regional indicator or flag emoji).
//! - Mongolian Free Variation Selectors (U+180B-U+180D) after a Mongolian
//!   letter (glyph selection, not a carrier).
//! - Khmer inherent vowels (U+17B4-U+17B5) after a Khmer consonant.
//! - Hangul jamo fillers (U+115F-U+1160) after Hangul jamo.
//! - Arabic/Syriac orthographic Cf marks (U+0600-U+0605 etc.).
//!
//! ## Performance
//!
//! Both [`inspect_text`] and [`clean_text`] run in O(n) time and O(k) extra
//! space, where n is the number of Unicode scalar values and k is the number
//! of distinct suspicious codepoints (typically a very small constant).
//!
//! ## Example
//! ```
//! use cum_rs::unicode::{clean_text, CleanOpts};
//!
//! let dirty = "Hello\u{200B} world\u{FEFF}!";
//! let opts = CleanOpts::safe();
//! let (clean, stats) = clean_text(dirty, &opts).unwrap();
//! assert_eq!(clean, "Hello world!");
//! assert_eq!(stats.removed_count, 2);
//! ```

use crate::types::{CharHit, CleanStats, Confidence, TextInspectReport, WatermarkKind};
use std::collections::BTreeMap;

/// Maximum input text length accepted by [`inspect_text`] and [`clean_text`].
///
/// 256 MiB expressed as a character count.  Because each Rust `char` is a
/// Unicode scalar value (32-bit), 256 MiB / 4 = 64 Mi chars maximum.
pub const MAX_TEXT_CHARS: usize = 64 * 1024 * 1024;

/// Unicode codepoints that are **always** stripped (invisible / format controls).
///
/// Every entry has no visible rendering and is used solely as an invisible
/// carrier when appearing outside its narrow orthographic context.
///
/// Time to construct: O(1): static data.
pub const STRIP_CODEPOINTS: &[u32] = &[
    0x00AD, // soft hyphen
    0x034F, // combining grapheme joiner
    0x061C, // Arabic letter mark (bidi)
    0x115F, // Hangul choseong filler (handled contextually below)
    0x1160, // Hangul jungseong filler
    0x17B4, // Khmer vowel inherent AQ
    0x17B5, // Khmer vowel inherent AA
    0x180B, // Mongolian free variation selector-1
    0x180C, // Mongolian free variation selector-2
    0x180D, // Mongolian free variation selector-3
    0x180E, // Mongolian vowel separator
    0x200B, // zero width space
    0x200C, // zero width non-joiner
    0x200D, // zero width joiner
    0x200E, // left-to-right mark
    0x200F, // right-to-left mark
    0x202A, // left-to-right embedding
    0x202B, // right-to-left embedding
    0x202C, // pop directional formatting
    0x202D, // left-to-right override
    0x202E, // right-to-left override
    0x2060, // word joiner
    0x2061, // function application
    0x2062, // invisible times
    0x2063, // invisible separator
    0x2064, // invisible plus
    0x2066, // left-to-right isolate
    0x2067, // right-to-left isolate
    0x2068, // first strong isolate
    0x2069, // pop directional isolate
    0x206A, // inhibit symmetric swapping
    0x206B, // activate symmetric swapping
    0x206C, // inhibit Arabic form shaping
    0x206D, // activate Arabic form shaping
    0x206E, // national digit shapes
    0x206F, // nominal digit shapes
    0xFEFF, // BOM / zero width no-break space
    0xFE00, // variation selector-1
    0xFE01, 0xFE02, 0xFE03, 0xFE04, 0xFE05, 0xFE06, 0xFE07, 0xFE08, 0xFE09, 0xFE0A, 0xFE0B, 0xFE0C,
    0xFE0D, 0xFE0E, 0xFE0F, // variation selector-16
    0xFFF9, // interlinear annotation anchor
    0xFFFA, // interlinear annotation separator
    0xFFFB, // interlinear annotation terminator
];

/// Space homoglyphs: Unicode codepoints that look like (or substitute for)
/// an ordinary ASCII space (U+0020).
///
/// Each entry is `(codepoint, replacement_char)`.
pub const SPACE_HOMOGLYPHS: &[(u32, char)] = &[
    (0x00A0, ' '), // no-break space
    (0x1680, ' '), // Ogham space mark
    (0x2000, ' '), // en quad
    (0x2001, ' '), // em quad
    (0x2002, ' '), // en space
    (0x2003, ' '), // em space
    (0x2004, ' '), // three-per-em space
    (0x2005, ' '), // four-per-em space
    (0x2006, ' '), // six-per-em space
    (0x2007, ' '), // figure space
    (0x2008, ' '), // punctuation space
    (0x2009, ' '), // thin space
    (0x200A, ' '), // hair space
    (0x202F, ' '), // narrow no-break space
    (0x205F, ' '), // medium mathematical space
    (0x3000, ' '), // ideographic space
];

/// Cyrillic and fullwidth Latin characters that are visually indistinguishable
/// from ASCII letters.
///
/// Only applied when [`CleanOpts::aggressive_confusables`] is `true`.
pub const LATIN_CONFUSABLES: &[(u32, char)] = &[
    (0x0410, 'A'), // Cyrillic А
    (0x0412, 'B'), // Cyrillic В
    (0x0415, 'E'), // Cyrillic Е
    (0x041A, 'K'), // Cyrillic К
    (0x041C, 'M'), // Cyrillic М
    (0x041D, 'H'), // Cyrillic Н
    (0x041E, 'O'), // Cyrillic О
    (0x0420, 'P'), // Cyrillic Р
    (0x0421, 'C'), // Cyrillic С
    (0x0422, 'T'), // Cyrillic Т
    (0x0425, 'X'), // Cyrillic Х
    (0x0430, 'a'), // Cyrillic а
    (0x0435, 'e'), // Cyrillic е
    (0x043E, 'o'), // Cyrillic о
    (0x0440, 'p'), // Cyrillic р
    (0x0441, 'c'), // Cyrillic с
    (0x0443, 'y'), // Cyrillic у
    (0x0445, 'x'), // Cyrillic х
    (0x0456, 'i'), // Cyrillic Ukrainian і
    // Fullwidth Latin A-Z
    (0xFF21, 'A'),
    (0xFF22, 'B'),
    (0xFF23, 'C'),
    (0xFF24, 'D'),
    (0xFF25, 'E'),
    (0xFF26, 'F'),
    (0xFF27, 'G'),
    (0xFF28, 'H'),
    (0xFF29, 'I'),
    (0xFF2A, 'J'),
    (0xFF2B, 'K'),
    (0xFF2C, 'L'),
    (0xFF2D, 'M'),
    (0xFF2E, 'N'),
    (0xFF2F, 'O'),
    (0xFF30, 'P'),
    (0xFF31, 'Q'),
    (0xFF32, 'R'),
    (0xFF33, 'S'),
    (0xFF34, 'T'),
    (0xFF35, 'U'),
    (0xFF36, 'V'),
    (0xFF37, 'W'),
    (0xFF38, 'X'),
    (0xFF39, 'Y'),
    (0xFF3A, 'Z'),
    // Fullwidth Latin a-z
    (0xFF41, 'a'),
    (0xFF42, 'b'),
    (0xFF43, 'c'),
    (0xFF44, 'd'),
    (0xFF45, 'e'),
    (0xFF46, 'f'),
    (0xFF47, 'g'),
    (0xFF48, 'h'),
    (0xFF49, 'i'),
    (0xFF4A, 'j'),
    (0xFF4B, 'k'),
    (0xFF4C, 'l'),
    (0xFF4D, 'm'),
    (0xFF4E, 'n'),
    (0xFF4F, 'o'),
    (0xFF50, 'p'),
    (0xFF51, 'q'),
    (0xFF52, 'r'),
    (0xFF53, 's'),
    (0xFF54, 't'),
    (0xFF55, 'u'),
    (0xFF56, 'v'),
    (0xFF57, 'w'),
    (0xFF58, 'x'),
    (0xFF59, 'y'),
    (0xFF5A, 'z'),
];

/// Bidirectional format control codepoints (subset of `STRIP_CODEPOINTS`).
const BIDI_CODEPOINTS: &[u32] = &[
    0x061C, 0x200E, 0x200F, 0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x2066, 0x2067, 0x2068, 0x2069,
];

/// Zero-width character family (common edit-based carriers).
const ZW_FAMILY: &[u32] = &[0x200B, 0x200C, 0x200D, 0x2060, 0xFEFF, 0x180E];

/// Orthographic Arabic/Syriac Cf marks that must always be preserved.
const ORTHOGRAPHIC_CF: &[u32] = &[
    0x0600, 0x0601, 0x0602, 0x0603, 0x0604, 0x0605, 0x06DD, 0x070F, 0x08E2, 0x110BD, 0x110CD,
];

/// Emoji glue codepoints: ZWJ and text/emoji variation selectors.
const EMOJI_GLUE: &[u32] = &[0x200D, 0xFE0E, 0xFE0F];

/// Script joiners: ZWNJ and ZWJ.
const SCRIPT_JOINERS: &[u32] = &[0x200C, 0x200D];

/// Mongolian Free Variation Selectors.
const MONGOLIAN_FVS: &[u32] = &[0x180B, 0x180C, 0x180D];

/// Khmer inherent vowels.
const KHMER_VOWELS: &[u32] = &[0x17B4, 0x17B5];

/// Hangul jamo fillers.
const HANGUL_FILLERS: &[u32] = &[0x115F, 0x1160];

/// Options for the [`inspect_text`] function.
#[derive(Debug, Clone, Default)]
pub struct InspectOpts {
    /// Include Cyrillic / fullwidth Latin confusable matches in findings.
    pub aggressive_confusables: bool,

    /// Strip emoji glue (ZWJ / VS after emoji base): paranoid mode.
    pub strip_emoji_glue: bool,
}

/// Options for the [`clean_text`] function.
#[derive(Debug, Clone, Default)]
pub struct CleanOpts {
    /// Normalize space homoglyphs to plain ASCII space.
    pub normalize_spaces: bool,

    /// Also replace Cyrillic / fullwidth Latin confusables.
    pub aggressive_confusables: bool,

    /// Apply NFKC normalization after cleaning.
    pub nfkc: bool,

    /// Strip emoji glue (ZWJ / VS after emoji base): paranoid mode.
    pub strip_emoji_glue: bool,
}

impl CleanOpts {
    /// Returns [`CleanOpts`] with safe defaults: space normalisation on,
    /// confusables off, no NFKC, emoji glue preserved.
    pub fn safe() -> Self {
        Self {
            normalize_spaces: true,
            aggressive_confusables: false,
            nfkc: false,
            strip_emoji_glue: false,
        }
    }
}

/// The decision made for a single character during inspection / cleaning.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Decision {
    /// Keep the character as-is.
    Keep,
    /// Remove the character entirely.
    Strip,
    /// Replace the character with a canonical equivalent.
    Replace(char),
}

/// Returns whether `cp` is a Unicode private-use area codepoint.
fn is_private_use(cp: u32) -> bool {
    (0xE000..=0xF8FF).contains(&cp)
        || (0xF0000..=0xFFFFD).contains(&cp)
        || (0x100000..=0x10FFFD).contains(&cp)
}

/// Returns whether `cp` must unconditionally be stripped (ignoring context).
fn is_strip_cp(cp: u32) -> bool {
    STRIP_CODEPOINTS.contains(&cp)
        || (0xE0100..=0xE01EF).contains(&cp) // VS17-VS256
        || (0xE0001..=0xE007F).contains(&cp) // tag characters
        || is_private_use(cp)
}

/// Classifies a strip-class codepoint into a [`WatermarkKind`].
fn strip_kind(cp: u32) -> WatermarkKind {
    if (0xE0001..=0xE007F).contains(&cp) {
        return WatermarkKind::TagChar;
    }
    if (0xE0100..=0xE01EF).contains(&cp)
        || (0xFE00..=0xFE0F).contains(&cp)
        || MONGOLIAN_FVS.contains(&cp)
    {
        return WatermarkKind::VariationSelector;
    }
    if BIDI_CODEPOINTS.contains(&cp) {
        return WatermarkKind::Bidi;
    }
    if ZW_FAMILY.contains(&cp) {
        return WatermarkKind::ZwjFamily;
    }
    if is_private_use(cp) {
        return WatermarkKind::PrivateUse;
    }
    WatermarkKind::UnicodeCarrier
}

/// Returns whether `cp` is an emoji base character.
///
/// Covers the Miscellaneous Symbols and Pictographs, Emoticons, Supplemental
/// Symbols, and a handful of well-known symbol codepoints.
fn is_emoji_base(cp: u32) -> bool {
    (0x1F000..=0x1FAFF).contains(&cp)
        || (0x2600..=0x27BF).contains(&cp)
        || (0x2B00..=0x2BFF).contains(&cp)
        || matches!(
            cp,
            0x00A9 | 0x00AE | 0x2122 | 0x3030 | 0x303D | 0x3297 | 0x3299
        )
        || ((0x0030..=0x0039).contains(&cp) || matches!(cp, 0x0023 | 0x002A))
}

/// Returns whether `cp` is a non-ASCII letter or mark (check for script-joiner
/// contextual preservation in complex scripts).
fn is_joining_letter(cp: u32) -> bool {
    if cp <= 0x7F {
        return false;
    }
    matches!(
        unicode_general_category::get_general_category(char::from_u32(cp).unwrap_or('\u{FFFD}')),
        unicode_general_category::GeneralCategory::LowercaseLetter
            | unicode_general_category::GeneralCategory::UppercaseLetter
            | unicode_general_category::GeneralCategory::TitlecaseLetter
            | unicode_general_category::GeneralCategory::OtherLetter
            | unicode_general_category::GeneralCategory::ModifierLetter
            | unicode_general_category::GeneralCategory::NonspacingMark
            | unicode_general_category::GeneralCategory::SpacingMark
            | unicode_general_category::GeneralCategory::EnclosingMark
    )
}

/// Returns whether `cp` is a Mongolian letter (for FVS context check).
fn is_mongolian_letter(cp: u32) -> bool {
    (0x1800..=0x18AF).contains(&cp)
        && matches!(
            unicode_general_category::get_general_category(
                char::from_u32(cp).unwrap_or('\u{FFFD}')
            ),
            unicode_general_category::GeneralCategory::LowercaseLetter
                | unicode_general_category::GeneralCategory::UppercaseLetter
                | unicode_general_category::GeneralCategory::OtherLetter
        )
}

/// Returns whether `cp` is a Khmer consonant or vowel-carrier (for inherent
/// vowel context check).
fn is_khmer_letter(cp: u32) -> bool {
    (0x1780..=0x17FF).contains(&cp)
        && matches!(
            unicode_general_category::get_general_category(
                char::from_u32(cp).unwrap_or('\u{FFFD}')
            ),
            unicode_general_category::GeneralCategory::LowercaseLetter
                | unicode_general_category::GeneralCategory::OtherLetter
        )
}

/// Returns whether `cp` is a Hangul jamo character (for filler context check).
fn is_hangul_jamo(cp: u32) -> bool {
    (0x1100..=0x11FF).contains(&cp)
        || (0xA960..=0xA97C).contains(&cp)
        || (0xD7B0..=0xD7C6).contains(&cp)
}

/// Returns whether a character is "glue": an invisible character that does
/// not advance the "previous kept" cursor during scanning.
///
/// Glue characters are contextually significant only when directly following
/// their base; consuming them does not break the chain for subsequent glue.
fn is_glue(cp: u32) -> bool {
    EMOJI_GLUE.contains(&cp)
        || SCRIPT_JOINERS.contains(&cp)
        || (0xE0020..=0xE007F).contains(&cp)
        || MONGOLIAN_FVS.contains(&cp)
        || KHMER_VOWELS.contains(&cp)
        || HANGUL_FILLERS.contains(&cp)
}

/// Classifies one input codepoint as `Keep`, `Strip`, or `Replace`.
///
/// `prev_kept` is the most recently kept non-glue character, used to
/// determine whether an invisible character is orthographically load-bearing.
///
/// Returns `(Decision, Option<WatermarkKind>)`: the kind is `Some` when the
/// character is suspicious, `None` when it is clean.
///
/// # Complexity
/// - Time: O(1) per character (`STRIP_CODEPOINTS.contains` on a small
///   static slice; all other checks are O(1) range/set checks).
/// - Space: O(1).
fn decide(
    ch: char,
    prev_kept: Option<char>,
    opts: &CleanOpts,
) -> (Decision, Option<WatermarkKind>) {
    let cp = ch as u32;

    if !opts.strip_emoji_glue {
        if EMOJI_GLUE.contains(&cp) && prev_kept.is_some_and(|prev| is_emoji_base(prev as u32)) {
            return (Decision::Keep, None);
        }
        if SCRIPT_JOINERS.contains(&cp)
            && prev_kept.is_some_and(|prev| is_joining_letter(prev as u32))
        {
            return (Decision::Keep, None);
        }
        if (0xE0020..=0xE007F).contains(&cp)
            && prev_kept.is_some_and(|prev| is_emoji_base(prev as u32))
        {
            return (Decision::Keep, None);
        }
        if MONGOLIAN_FVS.contains(&cp)
            && prev_kept.is_some_and(|prev| is_mongolian_letter(prev as u32))
        {
            return (Decision::Keep, None);
        }
        if KHMER_VOWELS.contains(&cp) && prev_kept.is_some_and(|prev| is_khmer_letter(prev as u32))
        {
            return (Decision::Keep, None);
        }
        if HANGUL_FILLERS.contains(&cp) && prev_kept.is_some_and(|prev| is_hangul_jamo(prev as u32))
        {
            return (Decision::Keep, None);
        }
        if ORTHOGRAPHIC_CF.contains(&cp) {
            return (Decision::Keep, None);
        }
    }

    if is_strip_cp(cp) {
        return (Decision::Strip, Some(strip_kind(cp)));
    }

    if opts.normalize_spaces {
        for &(homoglyph, replacement) in SPACE_HOMOGLYPHS {
            if cp == homoglyph {
                return (
                    Decision::Replace(replacement),
                    Some(WatermarkKind::SpaceHomoglyph),
                );
            }
        }
    }

    if opts.aggressive_confusables {
        for &(confusable, replacement) in LATIN_CONFUSABLES {
            if cp == confusable {
                return (
                    Decision::Replace(replacement),
                    Some(WatermarkKind::LatinConfusable),
                );
            }
        }
    }

    let cat = unicode_general_category::get_general_category(ch);
    if cat == unicode_general_category::GeneralCategory::Format {
        let is_space_homoglyph = SPACE_HOMOGLYPHS.iter().any(|&(c, _)| c == cp);
        if !is_space_homoglyph {
            return (Decision::Strip, Some(WatermarkKind::UnicodeCarrier));
        }
    }

    (Decision::Keep, None)
}

/// Returns a human-readable label for a Unicode codepoint.
fn char_label(cp: u32) -> String {
    if let Some(ch) = char::from_u32(cp) {
        let name = unicode_names2::name(ch)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let cat = unicode_general_category::get_general_category(ch);
        format!("U+{cp:04X} {name} ({cat:?})")
    } else {
        format!("U+{cp:04X} INVALID_CODEPOINT")
    }
}

/// Assigns a [`Confidence`] level to a Layer-A finding.
fn hit_confidence(kind: &WatermarkKind) -> Confidence {
    match kind {
        WatermarkKind::SpaceHomoglyph => Confidence::Informational,
        _ => Confidence::Probable,
    }
}

/// Inspects a text string for Layer-A Unicode watermark carriers.
///
/// Returns a [`TextInspectReport`] containing per-codepoint findings sorted
/// by descending occurrence count.  The input is scanned once; no clone of
/// the string is made.
///
/// # Arguments
/// * `text`: the text string to inspect.
/// * `opts`: inspection options ([`InspectOpts`]).
///
/// # Complexity
/// - Time: O(n): single pass over all Unicode scalar values.
/// - Space: O(k): one bucket per distinct suspicious codepoint.
///
/// # Errors
/// Returns an error if `text` exceeds [`MAX_TEXT_CHARS`].
pub fn inspect_text(text: &str, opts: &InspectOpts) -> crate::error::Result<TextInspectReport> {
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(crate::error::CumError::InputTooLarge {
            limit: MAX_TEXT_CHARS,
            actual: text.chars().count(),
        });
    }

    let clean_opts = CleanOpts {
        normalize_spaces: true,
        aggressive_confusables: opts.aggressive_confusables,
        nfkc: false,
        strip_emoji_glue: opts.strip_emoji_glue,
    };

    let mut buckets: BTreeMap<(u32, String), Vec<usize>> = BTreeMap::new();
    let mut prev_kept: Option<char> = None;

    for (char_offset, ch) in text.chars().enumerate() {
        let (decision, kind_opt) = decide(ch, prev_kept, &clean_opts);

        if let Some(kind) = kind_opt {
            let key = (ch as u32, kind.as_str().to_string());
            buckets.entry(key).or_default().push(char_offset);
            if let Decision::Replace(r) = decision
                && !is_glue(r as u32)
            {
                prev_kept = Some(r);
            }
        } else if matches!(decision, Decision::Keep) && !is_glue(ch as u32) {
            prev_kept = Some(ch);
        }
    }

    let mut hits: Vec<CharHit> = buckets
        .into_iter()
        .map(|((cp, kind_str), offsets)| {
            let kind = kind_from_str(&kind_str);
            let confidence = hit_confidence(&kind);
            let label = char_label(cp);
            let count = offsets.len();
            let sample_offsets = offsets.into_iter().take(10).collect();
            CharHit {
                codepoint: cp,
                character: char::from_u32(cp)
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
                label,
                count,
                kind,
                confidence,
                sample_offsets,
            }
        })
        .collect();

    hits.sort_by(|a, b| b.count.cmp(&a.count).then(a.codepoint.cmp(&b.codepoint)));

    let suspicious_total = hits.iter().map(|h| h.count).sum();
    let length = text.chars().count();

    let mut notes = vec![
        "Layer A only: invisible/format Unicode and space homoglyphs (edit-based carriers).".into(),
        "Statistical (token-sampling) watermarks are not detectable here; use Layer B rewrite.".into(),
        "Inspect kinds: zwj_family, bidi, tag_chars, variation_selector, private_use, space_homoglyph, latin_confusable, unicode_carrier.".into(),
        "Load-bearing invisibles are preserved by default: emoji glue (ZWJ/VS after emoji base), script joiners (ZWNJ/ZWJ inside complex scripts), flag tag chars, Mongolian FVS, Khmer inherent vowels, Hangul jamo fillers, orthographic Arabic/Syriac Cf marks.".into(),
    ];

    if hits.is_empty() {
        notes.push("No deterministic Layer A (invisible Unicode/format) carriers detected.".into());
    }

    Ok(TextInspectReport {
        length,
        suspicious_total,
        hits,
        notes,
    })
}

/// Removes all Layer-A Unicode watermark carriers from a text string.
///
/// Returns the cleaned string and a [`CleanStats`] summary.  The output
/// has at most the same length as the input; the function pre-allocates
/// the output buffer to avoid reallocation.
///
/// # Arguments
/// * `text`: the text string to clean.
/// * `opts`: cleaning options ([`CleanOpts`]).
///
/// # Complexity
/// - Time: O(n): single pass over all Unicode scalar values.
/// - Space: O(n): output buffer pre-allocated at `text.len()` bytes.
///
/// # Errors
/// Returns an error if `text` exceeds [`MAX_TEXT_CHARS`].
pub fn clean_text(text: &str, opts: &CleanOpts) -> crate::error::Result<(String, CleanStats)> {
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(crate::error::CumError::InputTooLarge {
            limit: MAX_TEXT_CHARS,
            actual: text.chars().count(),
        });
    }

    let mut out = String::with_capacity(text.len());
    let mut prev_kept: Option<char> = None;
    let mut removed_count: usize = 0;
    let mut replaced_count: usize = 0;
    let mut summary_items: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for ch in text.chars() {
        let (decision, kind_opt) = decide(ch, prev_kept, opts);
        match decision {
            Decision::Keep => {
                out.push(ch);
                if !is_glue(ch as u32) {
                    prev_kept = Some(ch);
                }
            }
            Decision::Strip => {
                removed_count += 1;
                if let Some(kind) = kind_opt {
                    *summary_items.entry(kind.as_str().to_string()).or_insert(0) += 1;
                }
            }
            Decision::Replace(replacement) => {
                out.push(replacement);
                replaced_count += 1;
                if let Some(kind) = kind_opt {
                    *summary_items.entry(kind.as_str().to_string()).or_insert(0) += 1;
                }
                if !is_glue(replacement as u32) {
                    prev_kept = Some(replacement);
                }
            }
        }
    }

    if opts.nfkc {
        let before_len = out.len();
        let normalized =
            unicode_normalization::UnicodeNormalization::nfkc(&*out).collect::<String>();
        if normalized.len() != before_len {
            replaced_count += normalized.len().abs_diff(before_len).max(1);
        }
        out = normalized;
    }

    let summary: Vec<String> = summary_items
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect();

    Ok((
        out,
        CleanStats {
            removed_count,
            replaced_count,
            metadata_chunks_removed: 0,
            summary,
        },
    ))
}

/// Converts a kind string back to a [`WatermarkKind`] (internal helper).
fn kind_from_str(s: &str) -> WatermarkKind {
    match s {
        "space_homoglyph" => WatermarkKind::SpaceHomoglyph,
        "latin_confusable" => WatermarkKind::LatinConfusable,
        "tag_chars" => WatermarkKind::TagChar,
        "variation_selector" => WatermarkKind::VariationSelector,
        "bidi" => WatermarkKind::Bidi,
        "zwj_family" => WatermarkKind::ZwjFamily,
        "private_use" => WatermarkKind::PrivateUse,
        _ => WatermarkKind::UnicodeCarrier,
    }
}
