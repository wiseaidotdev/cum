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

use phf::{Map, Set, phf_map, phf_set};
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
    "/usr/share/dict/english",
    "/usr/share/dict/words",
    "/usr/dict/words",
];

/// A compile-time perfect-hash set of English stop words.
///
/// Membership is tested in O(1) via a generated perfect hash: no heap allocation.
static STOP_WORDS: Set<&'static str> = phf_set! {
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
    "by", "is", "are", "was", "were", "be", "been", "have", "has", "had", "do", "does",
    "did", "will", "would", "could", "should", "may", "might", "shall", "its", "it",
    "this", "that", "these", "those", "not", "no", "nor", "so", "yet", "both", "also",
    "as", "if", "than", "then", "from", "into", "through", "during", "before", "after",
    "above", "below", "between", "each", "every", "any"
};

/// The curated synonym table compiled into the binary as a perfect hash map.
///
/// Each entry maps a common AI-generated word to a `&'static [&'static str]` slice of
/// semantically related alternatives drawn from plain English vocabulary.
///
/// Lookups are O(1) and require no heap allocation at runtime.
static CURATED_SYNONYMS: Map<&'static str, &'static [&'static str]> = phf_map! {
    "enables"       => &["allows", "permits", "empowers", "facilitates", "supports"],
    "represents"    => &["embodies", "symbolizes", "denotes", "signifies", "captures"],
    "describes"     => &["illustrates", "portrays", "articulates", "characterizes"],
    "manifests"     => &["reveals", "expresses", "demonstrates", "exhibits"],
    "connects"      => &["links", "unifies", "bridges", "binds", "joins"],
    "encodes"       => &["captures", "compresses", "maps", "stores", "embeds"],
    "defines"       => &["specifies", "determines", "establishes", "delineates"],
    "produces"      => &["generates", "yields", "creates", "constructs", "forms"],
    "transforms"    => &["converts", "shifts", "alters", "reconfigures", "reshapes"],
    "reveals"       => &["uncovers", "exposes", "discloses", "illuminates", "shows"],
    "governs"       => &["controls", "regulates", "directs", "commands", "shapes"],
    "expresses"     => &["articulates", "conveys", "communicates", "transmits"],
    "unveils"       => &["discloses", "reveals", "exposes", "uncovers", "presents"],
    "illuminates"   => &["clarifies", "enlightens", "elucidates", "reveals", "shows"],
    "shapes"        => &["molds", "forms", "structures", "defines", "guides"],
    "compresses"    => &["condenses", "distills", "reduces", "encapsulates"],
    "captures"      => &["encompasses", "embodies", "encapsulates", "reflects"],
    "generates"     => &["produces", "creates", "yields", "synthesizes", "builds"],
    "determines"    => &["establishes", "dictates", "defines", "resolves", "fixes"],
    "remains"       => &["persists", "endures", "abides", "continues", "stands"],
    "reflects"      => &["embodies", "mirrors", "represents", "signifies", "echoes"],
    "underlies"     => &["supports", "grounds", "anchors", "sustains", "informs"],
    "emerges"       => &["arises", "surfaces", "appears", "originates", "unfolds"],
    "constrains"    => &["limits", "bounds", "restricts", "confines", "regulates"],
    "encapsulates"  => &["contains", "embodies", "summarizes", "condenses"],
    "preserves"     => &["maintains", "sustains", "conserves", "upholds", "keeps"],
    "separates"     => &["divides", "partitions", "distinguishes", "isolates"],
    "truth"         => &["reality", "fact", "knowledge", "verity", "actuality"],
    "reality"       => &["existence", "actuality", "world", "nature", "truth"],
    "knowledge"     => &["understanding", "wisdom", "insight", "cognition", "learning"],
    "pattern"       => &["structure", "design", "configuration", "arrangement", "form"],
    "structure"     => &["framework", "organization", "architecture", "arrangement"],
    "symmetry"      => &["balance", "harmony", "proportion", "regularity", "order"],
    "entropy"       => &["disorder", "chaos", "uncertainty", "complexity", "randomness"],
    "energy"        => &["power", "force", "dynamics", "vitality", "potential"],
    "complexity"    => &["intricacy", "depth", "sophistication", "richness"],
    "harmony"       => &["balance", "coherence", "unity", "symmetry", "accord"],
    "balance"       => &["equilibrium", "harmony", "poise", "stability", "proportion"],
    "motion"        => &["movement", "dynamics", "flow", "trajectory", "progression"],
    "order"         => &["structure", "organization", "coherence", "arrangement"],
    "chaos"         => &["disorder", "turbulence", "randomness", "entropy", "flux"],
    "dimension"     => &["axis", "aspect", "realm", "domain", "magnitude"],
    "infinity"      => &["boundlessness", "endlessness", "vastness", "eternity"],
    "perception"    => &["awareness", "insight", "observation", "cognition", "sense"],
    "meaning"       => &["significance", "substance", "essence", "purpose", "value"],
    "existence"     => &["being", "reality", "presence", "life", "manifestation"],
    "universe"      => &["cosmos", "world", "reality", "existence", "totality"],
    "mathematics"   => &["algebra", "geometry", "calculus", "arithmetic", "analysis"],
    "equation"      => &["formula", "expression", "relationship", "model", "identity"],
    "frequency"     => &["resonance", "oscillation", "wavelength", "rhythm", "rate"],
    "resonance"     => &["harmony", "vibration", "coherence", "synchrony", "accord"],
    "evolution"     => &["transformation", "progression", "development", "dynamics"],
    "boundary"      => &["limit", "threshold", "barrier", "edge", "frontier"],
    "trajectory"    => &["path", "course", "orbit", "direction", "arc"],
    "causality"     => &["determinism", "consequence", "mechanism", "logic", "reason"],
    "logic"         => &["reasoning", "rationality", "inference", "deduction", "thought"],
    "force"         => &["energy", "power", "influence", "dynamics", "pressure"],
    "space"         => &["realm", "domain", "expanse", "field", "region"],
    "time"          => &["moment", "epoch", "duration", "continuity", "period"],
    "field"         => &["domain", "region", "space", "realm", "expanse"],
    "wave"          => &["oscillation", "vibration", "ripple", "undulation", "pulse"],
    "signal"        => &["indicator", "marker", "pattern", "trace", "message"],
    "computation"   => &["calculation", "processing", "evaluation", "analysis"],
    "simulation"    => &["modeling", "emulation", "representation", "approximation"],
    "prediction"    => &["forecast", "projection", "estimation", "inference"],
    "discovery"     => &["revelation", "finding", "insight", "breakthrough", "perception"],
    "foundation"    => &["basis", "grounding", "core", "bedrock", "principle"],
    "principle"     => &["law", "rule", "axiom", "tenet", "doctrine"],
    "intelligence"  => &["cognition", "awareness", "reasoning", "understanding"],
    "consciousness" => &["awareness", "perception", "cognition", "sentience"],
    "mathematical"  => &["symbolic", "geometric", "algebraic", "analytical", "formal"],
    "deterministic" => &["predictable", "systematic", "causal", "precise", "exact"],
    "probabilistic" => &["stochastic", "statistical", "uncertain", "random", "variable"],
    "infinite"      => &["boundless", "endless", "vast", "immeasurable", "limitless"],
    "fundamental"   => &["essential", "core", "primary", "foundational", "elemental"],
    "dynamic"       => &["evolving", "fluid", "active", "continuous", "adaptive"],
    "abstract"      => &["symbolic", "conceptual", "theoretical", "pure", "ideal"],
    "continuous"    => &["unbroken", "flowing", "perpetual", "sustained", "smooth"],
    "discrete"      => &["distinct", "separate", "finite", "quantized", "isolated"],
    "invariant"     => &["constant", "stable", "fixed", "unchanging", "conserved"],
    "coherent"      => &["unified", "consistent", "structured", "harmonious", "ordered"],
    "structural"    => &["architectural", "organizational", "systematic", "formal"],
    "axiomatic"     => &["foundational", "self-evident", "primary", "elemental"],
    "bounded"       => &["finite", "constrained", "limited", "contained", "restricted"],
    "symmetric"     => &["balanced", "regular", "uniform", "proportional", "equal"],
    "elegant"       => &["refined", "sophisticated", "beautiful", "graceful", "pure"],
    "precise"       => &["exact", "accurate", "rigorous", "meticulous", "definite"],
    "universal"     => &["general", "global", "absolute", "total", "pervasive"],
    "recursive"     => &["iterative", "self-referential", "repetitive", "cyclic"],
    "emergent"      => &["arising", "evolving", "developing", "unfolding", "appearing"],
    "complex"       => &["intricate", "sophisticated", "multifaceted", "rich", "deep"],
    "simple"        => &["elementary", "basic", "pure", "direct", "minimal"],
    "ancient"       => &["primordial", "prehistoric", "archaic", "classical", "timeless"],
    "sacred"        => &["divine", "revered", "hallowed", "eternal", "transcendent"],
    "cosmic"        => &["universal", "celestial", "infinite", "vast", "transcendent"],
    "hidden"        => &["concealed", "latent", "underlying", "subtle", "implicit"],
    "deeper"        => &["profound", "fundamental", "underlying", "essential", "core"],
    "new"           => &["novel", "emerging", "fresh", "modern", "innovative"],
    "pure"          => &["exact", "unadulterated", "precise", "fundamental", "essential"],
    "true"          => &["genuine", "authentic", "real", "valid", "accurate"],
    "great"         => &["profound", "vast", "significant", "remarkable", "extraordinary"],
    "vast"          => &["immense", "expansive", "boundless", "infinite", "enormous"],
    "known"         => &["established", "recognized", "understood", "observed", "verified"],
    "seen"          => &["observed", "perceived", "recognized", "witnessed", "noted"],
    "made"          => &["constructed", "formed", "created", "built", "composed"],
    "called"        => &["named", "termed", "labeled", "designated", "referred"],
    "use"           => &["employ", "apply", "utilize", "leverage", "adopt"],
    "used"          => &["employed", "applied", "utilized", "leveraged", "adopted"],
    "show"          => &["demonstrate", "display", "exhibit", "present", "reveal"],
    "shows"         => &["demonstrates", "displays", "exhibits", "presents", "reveals"],
    "allow"         => &["permit", "enable", "support", "facilitate", "authorize"],
    "allows"        => &["permits", "enables", "supports", "facilitates", "authorizes"],
    "provide"       => &["furnish", "supply", "offer", "deliver", "yield"],
    "provides"      => &["furnishes", "supplies", "offers", "delivers", "yields"],
    "ensure"        => &["guarantee", "verify", "confirm", "secure", "assure"],
    "ensures"       => &["guarantees", "verifies", "confirms", "secures", "assures"],
    "help"          => &["aid", "assist", "support", "facilitate", "guide"],
    "helps"         => &["aids", "assists", "supports", "facilitates", "guides"],
    "create"        => &["build", "construct", "form", "produce", "generate"],
    "creates"       => &["builds", "constructs", "forms", "produces", "generates"],
    "improve"       => &["enhance", "refine", "optimize", "advance", "elevate"],
    "improves"      => &["enhances", "refines", "optimizes", "advances", "elevates"],
    "important"     => &["significant", "critical", "essential", "vital", "key"],
    "significant"   => &["important", "notable", "substantial", "meaningful", "major"],
    "effective"     => &["efficient", "successful", "powerful", "productive", "capable"],
    "efficient"     => &["effective", "streamlined", "optimized", "productive", "capable"],
    "powerful"      => &["strong", "capable", "robust", "potent", "formidable"],
    "robust"        => &["strong", "reliable", "resilient", "sturdy", "durable"],
    "flexible"      => &["adaptable", "versatile", "agile", "malleable", "adjustable"],
    "comprehensive" => &["thorough", "complete", "exhaustive", "extensive", "detailed"],
    "innovative"    => &["novel", "creative", "pioneering", "original", "inventive"],
    "advanced"      => &["sophisticated", "developed", "elevated", "progressive", "refined"],
    "modern"        => &["contemporary", "current", "recent", "present-day", "up-to-date"],
    "various"       => &["diverse", "multiple", "different", "assorted", "varied"],
    "different"     => &["distinct", "varied", "diverse", "alternative", "separate"],
    "specific"      => &["particular", "precise", "exact", "definite", "explicit"],
    "general"       => &["broad", "overall", "widespread", "universal", "common"],
    "however"       => &["nevertheless", "nonetheless", "yet", "still", "though"],
    "therefore"     => &["thus", "hence", "consequently", "accordingly", "so"],
    "although"      => &["though", "while", "even", "despite", "notwithstanding"],
    "because"       => &["since", "given", "owing", "due", "resulting"],
    "since"         => &["because", "given", "as", "considering", "inasmuch"],
    "while"         => &["whereas", "although", "though", "during", "simultaneously"],
    "additionally"  => &["furthermore", "moreover", "also", "besides", "likewise"],
    "furthermore"   => &["moreover", "additionally", "also", "beyond", "likewise"],
    "moreover"      => &["furthermore", "additionally", "besides", "also", "beyond"],
    "specifically"  => &["particularly", "precisely", "explicitly", "exactly", "notably"],
    "particularly"  => &["specifically", "especially", "notably", "especially", "above"],
    "essentially"   => &["fundamentally", "basically", "primarily", "inherently", "chiefly"],
    "ultimately"    => &["finally", "fundamentally", "inherently", "at", "last"],
    "overall"       => &["broadly", "generally", "comprehensively", "wholistically", "total"],
    "approach"      => &["method", "strategy", "technique", "framework", "process"],
    "process"       => &["procedure", "method", "mechanism", "workflow", "pipeline"],
    "method"        => &["approach", "technique", "strategy", "procedure", "way"],
    "system"        => &["framework", "mechanism", "architecture", "platform", "arrangement"],
    "framework"     => &["structure", "system", "architecture", "foundation", "scaffold"],
    "solution"      => &["answer", "resolution", "remedy", "approach", "fix"],
    "challenge"     => &["difficulty", "obstacle", "problem", "hurdle", "complication"],
    "impact"        => &["effect", "influence", "consequence", "outcome", "result"],
    "result"        => &["outcome", "effect", "consequence", "product", "output"],
    "benefit"       => &["advantage", "gain", "merit", "value", "asset"],
    "advantage"     => &["benefit", "merit", "edge", "asset", "strength"],
    "capability"    => &["ability", "capacity", "power", "potential", "competence"],
    "performance"   => &["efficiency", "effectiveness", "operation", "execution", "output"],
    "analysis"      => &["examination", "study", "assessment", "evaluation", "investigation"],
    "data"          => &["information", "records", "metrics", "input", "evidence"],
    "context"       => &["setting", "background", "environment", "situation", "framework"],
    "information"   => &["data", "knowledge", "details", "facts", "content"],

    "sentence"      => &["phrase", "clause", "statement", "remark"],
    "identical"     => &["indistinguishable", "exact", "matching", "equivalent", "duplicate"],
    "replace"       => &["substitute", "swap", "exchange", "change", "alter"],
    "replaced"      => &["substituted", "swapped", "exchanged", "changed", "altered"],
    "avoid"         => &["prevent", "evade", "dodge", "bypass", "shun"],
    "may"           => &["might", "could", "can", "should"],
    "can"           => &["may", "could", "will", "might"],
    "option"        => &["setting", "choice", "parameter", "alternative", "preference"],
    "output"        => &["result", "product", "generation", "yield", "export"],
    "letters"       => &["characters", "symbols", "glyphs"],
    "letter"        => &["character", "symbol", "glyph"],
};

/// Loads the system word list grouped by word length for same-length substitution.
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
/// - **Tier 1**: Compile-time [`CURATED_SYNONYMS`] PHF map: O(1), zero allocation.
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
    by_length: HashMap<usize, Vec<String>>,
}

impl SynonymBank {
    /// Creates a new [`SynonymBank`], loading the system wordlist at construction time.
    ///
    /// The curated synonyms come from a compile-time PHF map (no runtime build cost).
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    /// let bank = SynonymBank::new();
    /// assert!(bank.curated_count() > 50);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w) where w is the number of words in the system dictionary.
    ///
    /// # Space Complexity
    ///
    /// O(w).
    pub fn new() -> Self {
        Self {
            by_length: load_wordlist_by_length(),
        }
    }

    /// Samples a random synonym for `word`.
    ///
    /// Tier 1 (curated PHF map) is consulted first.  If the word is absent from the
    /// curated table, Tier 2 (same-length wordlist) is used.
    ///
    /// # Arguments
    ///
    /// * `word` - The word to replace; lookup is case-insensitive.
    /// * `rng`  - A mutable random number generator.
    ///
    /// # Returns
    ///
    /// (`Option<String>`): A synonym string, or `None` when no alternative is available.
    ///
    /// # Time Complexity
    ///
    /// O(1) amortised for the PHF lookup; O(k) for sampling from the wordlist bucket
    /// where k is the number of words of the same length.
    ///
    /// # Space Complexity
    ///
    /// O(1) for the curated path; O(k) for the wordlist-bucket filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    ///
    /// let bank = SynonymBank::new();
    /// let mut rng = rand::rng();
    /// let syn = bank.candidate("chaos", &mut rng);
    /// assert!(syn.is_some()); // "chaos" is in the curated table
    /// ```
    pub fn candidate<R: Rng>(&self, word: &str, rng: &mut R) -> Option<String> {
        let lower = word.to_lowercase();

        if let Some(syns) = CURATED_SYNONYMS
            .get(lower.as_str())
            .filter(|s| !s.is_empty())
        {
            let idx = rng.random_range(0..syns.len());
            return Some(syns[idx].to_string());
        }

        if let Some(bucket) = self.by_length.get(&lower.len()) {
            let candidates: Vec<&String> = bucket.iter().filter(|w| w.as_str() != lower).collect();
            if !candidates.is_empty() {
                let idx = rng.random_range(0..candidates.len());
                return Some(candidates[idx].clone());
            }
        }

        None
    }

    /// Returns the number of entries in the curated synonym table.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    /// assert!(SynonymBank::new().curated_count() > 50);
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

    /// Returns the total number of words in the loaded system wordlist.
    ///
    /// May return 0 in environments where the system dictionary is unavailable
    /// (e.g. WASM, stripped CI containers).
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::SynonymBank;
    /// let _ = SynonymBank::new().wordlist_len(); // may be 0 in CI
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(b) where b is the number of distinct word-length buckets.
    pub fn wordlist_len(&self) -> usize {
        self.by_length.values().map(|v| v.len()).sum()
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
    /// The synonym bank used for word lookups.
    bank: SynonymBank,
    /// Per-word substitution probability, clamped to `[0.0, 1.0]`.
    probability: f64,
}

impl StochasticEnhancer {
    /// Creates a [`StochasticEnhancer`] with a custom substitution probability.
    ///
    /// The value is clamped to `[0.0, 1.0]` before use.
    ///
    /// # Arguments
    ///
    /// * `probability` - Per-word substitution probability in `[0.0, 1.0]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    /// let e = StochasticEnhancer::new(0.3);
    /// assert_eq!(e.probability(), 0.3);
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(w) at construction (wordlist loading); O(1) thereafter.
    ///
    /// # Space Complexity
    ///
    /// O(w) where w is the size of the system wordlist.
    pub fn new(probability: f64) -> Self {
        Self {
            bank: SynonymBank::new(),
            probability: probability.clamp(0.0, 1.0),
        }
    }

    /// Creates a [`StochasticEnhancer`] with `p = 0.5`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    /// assert_eq!(StochasticEnhancer::with_default_probability().probability(), 0.5);
    /// ```
    pub fn with_default_probability() -> Self {
        Self::new(DEFAULT_REPLACEMENT_PROBABILITY)
    }

    /// Returns the configured substitution probability.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    /// assert_eq!(StochasticEnhancer::new(0.7).probability(), 0.7);
    /// ```
    pub fn probability(&self) -> f64 {
        self.probability
    }

    /// Enhances all lines of `text` by stochastic synonym substitution.
    ///
    /// Line breaks are preserved.  Each line is processed independently so that
    /// the random stream does not bleed across line boundaries.
    ///
    /// # Arguments
    ///
    /// * `text` - Input text (may contain newlines).
    ///
    /// # Returns
    ///
    /// An [`EnhanceOutput`] containing the enhanced text, the probability, and the
    /// count of words that were substituted.
    ///
    /// # Examples
    ///
    /// ```
    /// use cum_rs::stochastic::StochasticEnhancer;
    ///
    /// let e   = StochasticEnhancer::new(1.0); // always substitute when possible
    /// let out = e.enhance("chaos\nenergy");
    /// assert_eq!(out.text.lines().count(), 2); // line structure preserved
    /// ```
    ///
    /// # Time Complexity
    ///
    /// O(t) where t is the number of whitespace-separated tokens in `text`.
    ///
    /// # Space Complexity
    ///
    /// O(t) for the output buffer.
    pub fn enhance(&self, text: &str) -> EnhanceOutput {
        let mut rng = thread_rng();
        let mut total_substituted: usize = 0;

        let enhanced = text
            .split('\n')
            .map(|line| {
                let (line_text, substituted) = self.enhance_line(line, &mut rng);
                total_substituted += substituted;
                line_text
            })
            .collect::<Vec<_>>()
            .join("\n");

        EnhanceOutput {
            text: enhanced,
            probability: self.probability,
            words_substituted: total_substituted,
        }
    }

    /// Enhances a single line by processing tokens left-to-right.
    ///
    /// Returns the enhanced line and the count of substituted words.
    ///
    /// Stop words are never substituted.  Case style of the original token is
    /// applied to the replacement.
    ///
    /// # Time Complexity
    ///
    /// O(k) where k is the number of whitespace-separated tokens in `line`.
    ///
    /// # Space Complexity
    ///
    /// O(k).
    fn enhance_line<R: Rng>(&self, line: &str, rng: &mut R) -> (String, usize) {
        let mut result: Vec<String> = Vec::new();
        let mut is_first = true;
        let mut substituted: usize = 0;

        for raw in line.split_whitespace() {
            let (prefix, core, suffix) = split_token(raw);
            let lower = core.to_lowercase();
            let is_stop = STOP_WORDS.contains(lower.as_str());
            let was_capitalized = core
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            let was_all_caps = !core.is_empty() && core.chars().all(|c| c.is_uppercase());

            let replacement = if !is_stop && !core.is_empty() && rng.random_bool(self.probability) {
                match self.bank.candidate(&lower, rng) {
                    Some(syn) => {
                        substituted += 1;
                        if was_all_caps {
                            syn.to_uppercase()
                        } else if was_capitalized || is_first {
                            capitalize(&syn)
                        } else {
                            syn
                        }
                    }
                    None => core.to_string(),
                }
            } else {
                core.to_string()
            };

            result.push(format!("{}{}{}", prefix, replacement, suffix));
            is_first = false;
        }

        (result.join(" "), substituted)
    }
}

/// Splits a raw token such as `"(word,"` into `("(", "word", ",")`.
///
/// Leading punctuation/symbols are placed in `prefix`; trailing punctuation in `suffix`;
/// the alphabetic core in the middle.
///
/// # Arguments
///
/// * `raw` - A whitespace-separated token from the input text.
///
/// # Returns
///
/// (`(&str, &str, &str)`): `(prefix, core, suffix)` where `core` is the alphabetic span.
///
/// # Time Complexity
///
/// O(n) where n is the byte length of `raw`.
///
/// # Space Complexity
///
/// O(1): returns slices into the original string.
pub fn split_token(raw: &str) -> (&str, &str, &str) {
    let prefix_end = raw.find(|c: char| c.is_alphabetic()).unwrap_or(raw.len());
    let content_start = prefix_end;
    if content_start >= raw.len() {
        return ("", raw, "");
    }
    let suffix_start = raw[content_start..]
        .rfind(|c: char| c.is_alphabetic())
        .map(|i| {
            content_start
                + i
                + raw[content_start + i..]
                    .chars()
                    .next()
                    .map(|ch| ch.len_utf8())
                    .unwrap_or(1)
        })
        .unwrap_or(raw.len());
    (
        &raw[..content_start],
        &raw[content_start..suffix_start],
        &raw[suffix_start..],
    )
}

/// Uppercases the first character of `s` and leaves the rest unchanged.
///
/// # Arguments
///
/// * `s` - The input string slice.
///
/// # Returns
///
/// (`String`): The string with its first character uppercased.
///
/// # Examples
///
/// ```
/// use cum_rs::stochastic::capitalize;
///
/// assert_eq!(capitalize("hello"), "Hello");
/// assert_eq!(capitalize(""),      "");
/// assert_eq!(capitalize("a"),     "A");
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
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Returns `true` when `word` is an English stop word according to the compile-time
/// PHF set.
///
/// The lookup is O(1) with no heap allocation.
///
/// # Arguments
///
/// * `word` - A lowercase word (lookup is case-sensitive against the PHF set).
///
/// # Returns
///
/// (`bool`): `true` iff the word is in the stop-word set.
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
/// O(1).
///
/// # Space Complexity
///
/// O(1).
pub fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(word)
}
