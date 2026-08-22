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
//! | Braille blank | U+2800 (invisible carrier) | Strip |
//! | Space homoglyphs | En-Quad, Hair Space, Narrow NBSP, … → U+0020 | Replace |
//! | Dash / hyphen homoglyphs | U+2010-U+2015, U+2011 (non-breaking hyphen), U+2212, U+FE58, U+FE63, U+FF0D → U+002D | Replace |
//! | Punctuation homoglyphs | Curly quotes, ellipsis, prime marks, fraction slash, … | Replace |
//! | Cyrillic/fullwidth Latin confusables | Cyr А→A, FF21→A, … (aggressive mode) | Replace |
//! | Mathematical Alphanumeric Symbols | U+1D400-U+1D7FF 𝐀→A, 𝐚→a, 𝟎→0 (aggressive mode) | Replace |
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
//!
//! # See Also
//!
//! - [Unicode Technical Standard #39: Unicode Security Mechanisms](https://www.unicode.org/reports/tr39/) - details the visual confusables and space homoglyphs handled by the `aggressive_confusables` flag.
//! - [Unicode Standard Annex #9: Unicode Bidirectional Algorithm](https://www.unicode.org/reports/tr9/) - the specification for the invisible BIDI formatting controls stripped by this module.
//! - [Unicode Character Database - Confusables](https://www.unicode.org/Public/security/latest/confusables.txt) - the source for the dash, punctuation, and mathematical alphanumeric confusable tables.

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
    0x2800, // Braille blank (invisible; used as steganographic carrier)
    0xFEFF, // BOM / zero width no-break space
    0xFE00, // variation selector-1
    0xFE01, 0xFE02, 0xFE03, 0xFE04, 0xFE05, 0xFE06, 0xFE07, 0xFE08, 0xFE09, 0xFE0A, 0xFE0B, 0xFE0C,
    0xFE0D, 0xFE0E, 0xFE0F, // variation selector-16
    0xFFF9, // interlinear annotation anchor
    0xFFFA, // interlinear annotation separator
    0xFFFB, // interlinear annotation terminator
];

/// Unicode dash and hyphen homoglyphs that should be replaced with ASCII
/// hyphen-minus (U+002D).
///
/// Covers every Unicode character whose primary visual appearance is a
/// horizontal stroke of hyphen/dash width, excluding the em-dash and en-dash
/// used for intentional prose punctuation (those appear in
/// [`PUNCTUATION_HOMOGLYPHS`] with their own ASCII mappings).
///
/// The non-breaking hyphen U+2011 (‑) is the most common invisible-watermark
/// carrier in this family: it renders identically to U+002D in most fonts
/// but breaks substring searches and triggers invisible divergence in diff
/// tools.
///
/// Time to construct: O(1): static data.
pub const DASH_HOMOGLYPHS: &[(u32, char)] = &[
    (0x2010, '-'), // hyphen (U+2010)
    (0x2011, '-'), // non-breaking hyphen (U+2011) ← primary carrier
    (0x2012, '-'), // figure dash
    (0x2013, '-'), // en dash
    (0x2014, '-'), // em dash
    (0x2015, '-'), // horizontal bar
    (0x2212, '-'), // minus sign
    (0x2796, '-'), // heavy minus sign (emoji)
    (0xFE58, '-'), // small em dash
    (0xFE63, '-'), // small hyphen-minus
    (0xFF0D, '-'), // fullwidth hyphen-minus
];

/// Unicode punctuation homoglyphs that are replaced with their plain ASCII
/// equivalents when [`CleanOpts::normalize_punctuation`] is enabled.
///
/// Covers typographic quotes, apostrophes, ellipsis, prime marks, angle
/// quotes, and other punctuation characters that LLMs routinely substitute
/// for their plain-ASCII counterparts, creating invisible byte-level
/// divergence without visible difference.
///
/// Time to construct: O(1): static data.
pub const PUNCTUATION_HOMOGLYPHS: &[(u32, &str)] = &[
    (0x2018, "'"),    // left single quotation mark
    (0x2019, "'"),    // right single quotation mark / apostrophe
    (0x201A, "'"),    // single low-9 quotation mark
    (0x201B, "'"),    // single high-reversed-9 quotation mark
    (0x201C, "\""),   // left double quotation mark
    (0x201D, "\""),   // right double quotation mark
    (0x201E, "\""),   // double low-9 quotation mark
    (0x201F, "\""),   // double high-reversed-9 quotation mark
    (0x2024, "."),    // one dot leader
    (0x2025, ".."),   // two dot leader
    (0x2026, "..."),  // horizontal ellipsis
    (0x2032, "'"),    // prime (used as apostrophe)
    (0x2033, "\"\""), // double prime
    (0x2035, "'"),    // reversed prime
    (0x2039, "<"),    // single left-pointing angle quotation mark
    (0x203A, ">"),    // single right-pointing angle quotation mark
    (0x00AB, "<<"),   // left-pointing double angle quotation mark
    (0x00BB, ">>"),   // right-pointing double angle quotation mark
    (0x2044, "/"),    // fraction slash
    (0x2215, "/"),    // division slash
    (0xFF01, "!"),    // fullwidth exclamation mark
    (0xFF02, "\""),   // fullwidth quotation mark
    (0xFF07, "'"),    // fullwidth apostrophe
    (0xFF08, "("),    // fullwidth left parenthesis
    (0xFF09, ")"),    // fullwidth right parenthesis
    (0xFF0C, ","),    // fullwidth comma
    (0xFF0E, "."),    // fullwidth full stop
    (0xFF1A, ":"),    // fullwidth colon
    (0xFF1B, ";"),    // fullwidth semicolon
    (0xFF1F, "?"),    // fullwidth question mark
    (0xFF3B, "["),    // fullwidth left square bracket
    (0xFF3D, "]"),    // fullwidth right square bracket
    (0xFF3F, "_"),    // fullwidth low line
    (0xFF5B, "{"),    // fullwidth left curly bracket
    (0xFF5D, "}"),    // fullwidth right curly bracket
];

/// Mathematical Alphanumeric Symbols (U+1D400-U+1D7FF) mapped to their ASCII
/// equivalents, applied only when [`CleanOpts::aggressive_confusables`] is
/// `true`.
///
/// These symbols (𝐀, 𝒂, 𝟎, etc.) have identical glyphs to their base ASCII
/// characters in virtually every rendering context, making them effective
/// invisible substitution carriers.
///
/// Time to construct: O(1): static data.
pub const MATH_ALPHANUM_CONFUSABLES: &[(u32, char)] = &[
    // Mathematical Bold Capital A-Z
    (0x1D400, 'A'),
    (0x1D401, 'B'),
    (0x1D402, 'C'),
    (0x1D403, 'D'),
    (0x1D404, 'E'),
    (0x1D405, 'F'),
    (0x1D406, 'G'),
    (0x1D407, 'H'),
    (0x1D408, 'I'),
    (0x1D409, 'J'),
    (0x1D40A, 'K'),
    (0x1D40B, 'L'),
    (0x1D40C, 'M'),
    (0x1D40D, 'N'),
    (0x1D40E, 'O'),
    (0x1D40F, 'P'),
    (0x1D410, 'Q'),
    (0x1D411, 'R'),
    (0x1D412, 'S'),
    (0x1D413, 'T'),
    (0x1D414, 'U'),
    (0x1D415, 'V'),
    (0x1D416, 'W'),
    (0x1D417, 'X'),
    (0x1D418, 'Y'),
    (0x1D419, 'Z'),
    // Mathematical Bold Small a-z
    (0x1D41A, 'a'),
    (0x1D41B, 'b'),
    (0x1D41C, 'c'),
    (0x1D41D, 'd'),
    (0x1D41E, 'e'),
    (0x1D41F, 'f'),
    (0x1D420, 'g'),
    (0x1D421, 'h'),
    (0x1D422, 'i'),
    (0x1D423, 'j'),
    (0x1D424, 'k'),
    (0x1D425, 'l'),
    (0x1D426, 'm'),
    (0x1D427, 'n'),
    (0x1D428, 'o'),
    (0x1D429, 'p'),
    (0x1D42A, 'q'),
    (0x1D42B, 'r'),
    (0x1D42C, 's'),
    (0x1D42D, 't'),
    (0x1D42E, 'u'),
    (0x1D42F, 'v'),
    (0x1D430, 'w'),
    (0x1D431, 'x'),
    (0x1D432, 'y'),
    (0x1D433, 'z'),
    // Mathematical Italic Capital A-Z
    (0x1D434, 'A'),
    (0x1D435, 'B'),
    (0x1D436, 'C'),
    (0x1D437, 'D'),
    (0x1D438, 'E'),
    (0x1D439, 'F'),
    (0x1D43A, 'G'),
    (0x1D43B, 'H'),
    (0x1D43C, 'I'),
    (0x1D43D, 'J'),
    (0x1D43E, 'K'),
    (0x1D43F, 'L'),
    (0x1D440, 'M'),
    (0x1D441, 'N'),
    (0x1D442, 'O'),
    (0x1D443, 'P'),
    (0x1D444, 'Q'),
    (0x1D445, 'R'),
    (0x1D446, 'S'),
    (0x1D447, 'T'),
    (0x1D448, 'U'),
    (0x1D449, 'V'),
    (0x1D44A, 'W'),
    (0x1D44B, 'X'),
    (0x1D44C, 'Y'),
    (0x1D44D, 'Z'),
    // Mathematical Italic Small a-z (note: h=U+210E planck, omitted; i,j omitted)
    (0x1D44E, 'a'),
    (0x1D44F, 'b'),
    (0x1D450, 'c'),
    (0x1D451, 'd'),
    (0x1D452, 'e'),
    (0x1D453, 'f'),
    (0x1D454, 'g'),
    (0x1D456, 'i'),
    (0x1D457, 'j'),
    (0x1D458, 'k'),
    (0x1D459, 'l'),
    (0x1D45A, 'm'),
    (0x1D45B, 'n'),
    (0x1D45C, 'o'),
    (0x1D45D, 'p'),
    (0x1D45E, 'q'),
    (0x1D45F, 'r'),
    (0x1D460, 's'),
    (0x1D461, 't'),
    (0x1D462, 'u'),
    (0x1D463, 'v'),
    (0x1D464, 'w'),
    (0x1D465, 'x'),
    (0x1D466, 'y'),
    (0x1D467, 'z'),
    // Mathematical Bold Italic Capital A-Z
    (0x1D468, 'A'),
    (0x1D469, 'B'),
    (0x1D46A, 'C'),
    (0x1D46B, 'D'),
    (0x1D46C, 'E'),
    (0x1D46D, 'F'),
    (0x1D46E, 'G'),
    (0x1D46F, 'H'),
    (0x1D470, 'I'),
    (0x1D471, 'J'),
    (0x1D472, 'K'),
    (0x1D473, 'L'),
    (0x1D474, 'M'),
    (0x1D475, 'N'),
    (0x1D476, 'O'),
    (0x1D477, 'P'),
    (0x1D478, 'Q'),
    (0x1D479, 'R'),
    (0x1D47A, 'S'),
    (0x1D47B, 'T'),
    (0x1D47C, 'U'),
    (0x1D47D, 'V'),
    (0x1D47E, 'W'),
    (0x1D47F, 'X'),
    (0x1D480, 'Y'),
    (0x1D481, 'Z'),
    // Mathematical Bold Italic Small a-z
    (0x1D482, 'a'),
    (0x1D483, 'b'),
    (0x1D484, 'c'),
    (0x1D485, 'd'),
    (0x1D486, 'e'),
    (0x1D487, 'f'),
    (0x1D488, 'g'),
    (0x1D489, 'h'),
    (0x1D48A, 'i'),
    (0x1D48B, 'j'),
    (0x1D48C, 'k'),
    (0x1D48D, 'l'),
    (0x1D48E, 'm'),
    (0x1D48F, 'n'),
    (0x1D490, 'o'),
    (0x1D491, 'p'),
    (0x1D492, 'q'),
    (0x1D493, 'r'),
    (0x1D494, 's'),
    (0x1D495, 't'),
    (0x1D496, 'u'),
    (0x1D497, 'v'),
    (0x1D498, 'w'),
    (0x1D499, 'x'),
    (0x1D49A, 'y'),
    (0x1D49B, 'z'),
    // Mathematical Sans-Serif Bold Capital A-Z
    (0x1D5D4, 'A'),
    (0x1D5D5, 'B'),
    (0x1D5D6, 'C'),
    (0x1D5D7, 'D'),
    (0x1D5D8, 'E'),
    (0x1D5D9, 'F'),
    (0x1D5DA, 'G'),
    (0x1D5DB, 'H'),
    (0x1D5DC, 'I'),
    (0x1D5DD, 'J'),
    (0x1D5DE, 'K'),
    (0x1D5DF, 'L'),
    (0x1D5E0, 'M'),
    (0x1D5E1, 'N'),
    (0x1D5E2, 'O'),
    (0x1D5E3, 'P'),
    (0x1D5E4, 'Q'),
    (0x1D5E5, 'R'),
    (0x1D5E6, 'S'),
    (0x1D5E7, 'T'),
    (0x1D5E8, 'U'),
    (0x1D5E9, 'V'),
    (0x1D5EA, 'W'),
    (0x1D5EB, 'X'),
    (0x1D5EC, 'Y'),
    (0x1D5ED, 'Z'),
    // Mathematical Sans-Serif Bold Small a-z
    (0x1D5EE, 'a'),
    (0x1D5EF, 'b'),
    (0x1D5F0, 'c'),
    (0x1D5F1, 'd'),
    (0x1D5F2, 'e'),
    (0x1D5F3, 'f'),
    (0x1D5F4, 'g'),
    (0x1D5F5, 'h'),
    (0x1D5F6, 'i'),
    (0x1D5F7, 'j'),
    (0x1D5F8, 'k'),
    (0x1D5F9, 'l'),
    (0x1D5FA, 'm'),
    (0x1D5FB, 'n'),
    (0x1D5FC, 'o'),
    (0x1D5FD, 'p'),
    (0x1D5FE, 'q'),
    (0x1D5FF, 'r'),
    (0x1D600, 's'),
    (0x1D601, 't'),
    (0x1D602, 'u'),
    (0x1D603, 'v'),
    (0x1D604, 'w'),
    (0x1D605, 'x'),
    (0x1D606, 'y'),
    (0x1D607, 'z'),
    // Mathematical Monospace Capital A-Z
    (0x1D670, 'A'),
    (0x1D671, 'B'),
    (0x1D672, 'C'),
    (0x1D673, 'D'),
    (0x1D674, 'E'),
    (0x1D675, 'F'),
    (0x1D676, 'G'),
    (0x1D677, 'H'),
    (0x1D678, 'I'),
    (0x1D679, 'J'),
    (0x1D67A, 'K'),
    (0x1D67B, 'L'),
    (0x1D67C, 'M'),
    (0x1D67D, 'N'),
    (0x1D67E, 'O'),
    (0x1D67F, 'P'),
    (0x1D680, 'Q'),
    (0x1D681, 'R'),
    (0x1D682, 'S'),
    (0x1D683, 'T'),
    (0x1D684, 'U'),
    (0x1D685, 'V'),
    (0x1D686, 'W'),
    (0x1D687, 'X'),
    (0x1D688, 'Y'),
    (0x1D689, 'Z'),
    // Mathematical Monospace Small a-z
    (0x1D68A, 'a'),
    (0x1D68B, 'b'),
    (0x1D68C, 'c'),
    (0x1D68D, 'd'),
    (0x1D68E, 'e'),
    (0x1D68F, 'f'),
    (0x1D690, 'g'),
    (0x1D691, 'h'),
    (0x1D692, 'i'),
    (0x1D693, 'j'),
    (0x1D694, 'k'),
    (0x1D695, 'l'),
    (0x1D696, 'm'),
    (0x1D697, 'n'),
    (0x1D698, 'o'),
    (0x1D699, 'p'),
    (0x1D69A, 'q'),
    (0x1D69B, 'r'),
    (0x1D69C, 's'),
    (0x1D69D, 't'),
    (0x1D69E, 'u'),
    (0x1D69F, 'v'),
    (0x1D6A0, 'w'),
    (0x1D6A1, 'x'),
    (0x1D6A2, 'y'),
    (0x1D6A3, 'z'),
    // Mathematical Bold Digits 0-9
    (0x1D7CE, '0'),
    (0x1D7CF, '1'),
    (0x1D7D0, '2'),
    (0x1D7D1, '3'),
    (0x1D7D2, '4'),
    (0x1D7D3, '5'),
    (0x1D7D4, '6'),
    (0x1D7D5, '7'),
    (0x1D7D6, '8'),
    (0x1D7D7, '9'),
    // Mathematical Sans-Serif Bold Digits 0-9
    (0x1D7EC, '0'),
    (0x1D7ED, '1'),
    (0x1D7EE, '2'),
    (0x1D7EF, '3'),
    (0x1D7F0, '4'),
    (0x1D7F1, '5'),
    (0x1D7F2, '6'),
    (0x1D7F3, '7'),
    (0x1D7F4, '8'),
    (0x1D7F5, '9'),
    // Mathematical Monospace Digits 0-9
    (0x1D7F6, '0'),
    (0x1D7F7, '1'),
    (0x1D7F8, '2'),
    (0x1D7F9, '3'),
    (0x1D7FA, '4'),
    (0x1D7FB, '5'),
    (0x1D7FC, '6'),
    (0x1D7FD, '7'),
    (0x1D7FE, '8'),
    (0x1D7FF, '9'),
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

    /// Include dash and punctuation homoglyph matches in findings.
    pub normalize_punctuation: bool,

    /// Strip emoji glue (ZWJ / VS after emoji base): paranoid mode.
    pub strip_emoji_glue: bool,
}

/// Options for the [`clean_text`] function.
#[derive(Debug, Clone, Default)]
pub struct CleanOpts {
    /// Normalize space homoglyphs to plain ASCII space.
    pub normalize_spaces: bool,

    /// Replace Cyrillic, fullwidth Latin, and Mathematical Alphanumeric
    /// Symbol confusables with their ASCII equivalents.
    pub aggressive_confusables: bool,

    /// Replace dash/hyphen homoglyphs (U+2010-U+2015, U+2011, U+2212, …)
    /// with ASCII hyphen-minus, and replace typographic punctuation (curly
    /// quotes, ellipsis, …) with their plain ASCII equivalents.
    pub normalize_punctuation: bool,

    /// Apply NFKC normalization after cleaning.
    pub nfkc: bool,

    /// Strip emoji glue (ZWJ / VS after emoji base): paranoid mode.
    pub strip_emoji_glue: bool,
}

impl CleanOpts {
    /// Returns [`CleanOpts`] with safe defaults: space normalisation and
    /// punctuation normalisation on; confusables off, no NFKC, emoji glue
    /// preserved.
    pub fn safe() -> Self {
        Self {
            normalize_spaces: true,
            aggressive_confusables: false,
            normalize_punctuation: true,
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
    /// Replace the character with a single canonical equivalent.
    Replace(char),
    /// Replace the character with a multi-character ASCII string.
    ReplaceStr(&'static str),
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

    if opts.normalize_punctuation {
        for &(homoglyph, replacement) in DASH_HOMOGLYPHS {
            if cp == homoglyph {
                return (
                    Decision::Replace(replacement),
                    Some(WatermarkKind::DashHomoglyph),
                );
            }
        }
        for &(homoglyph, replacement_str) in PUNCTUATION_HOMOGLYPHS {
            if cp == homoglyph {
                if let Some(replacement_char) = replacement_str
                    .chars()
                    .next()
                    .filter(|_| replacement_str.chars().count() == 1)
                {
                    return (
                        Decision::Replace(replacement_char),
                        Some(WatermarkKind::PunctuationHomoglyph),
                    );
                }
                return (
                    Decision::ReplaceStr(replacement_str),
                    Some(WatermarkKind::PunctuationHomoglyph),
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
        for &(confusable, replacement) in MATH_ALPHANUM_CONFUSABLES {
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
        let is_dash_homoglyph = DASH_HOMOGLYPHS.iter().any(|&(c, _)| c == cp);
        if !is_space_homoglyph && !is_dash_homoglyph {
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
        normalize_punctuation: opts.normalize_punctuation,
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
            match &decision {
                Decision::Replace(r) if !is_glue(*r as u32) => {
                    prev_kept = Some(*r);
                }
                Decision::ReplaceStr(s) => {
                    prev_kept = s.chars().last();
                }
                _ => {}
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
            Decision::ReplaceStr(replacement_str) => {
                out.push_str(replacement_str);
                replaced_count += 1;
                if let Some(kind) = kind_opt {
                    *summary_items.entry(kind.as_str().to_string()).or_insert(0) += 1;
                }
                prev_kept = replacement_str.chars().last();
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
        "dash_homoglyph" => WatermarkKind::DashHomoglyph,
        "punctuation_homoglyph" => WatermarkKind::PunctuationHomoglyph,
        _ => WatermarkKind::UnicodeCarrier,
    }
}
