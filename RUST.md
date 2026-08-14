# Rust Usage Guide: `cum-rs`

## Installation

```toml
[dependencies]
cum-rs = "0.1.0"
```

## Text Cleaning (Layer A)

```rust
use cum_rs::unicode::{clean_text, CleanOpts};

let dirty = "Hello\u{200B} world\u{FEFF}!";
let opts = CleanOpts::safe();
let (clean, stats) = clean_text(dirty, &opts).unwrap();
assert_eq!(clean, "Hello world!");
println!("Removed: {}", stats.removed_count);
```

## Text Inspection

```rust
use cum_rs::unicode::{inspect_text, InspectOpts};

let report = inspect_text("Hello\u{200B}!", &InspectOpts::default()).unwrap();
for hit in &report.hits {
    println!("{}: {} occurrences ({})", hit.label, hit.count, hit.confidence.as_str());
}
```

## Image Metadata Stripping

```rust
use cum_rs::image_meta::clean_image;
use std::fs;

let png = fs::read("input.png").unwrap();
let cleaned = clean_image(&png).unwrap();
fs::write("output.png", &cleaned).unwrap();
```

## Unified API (auto-detect format)

```rust
use cum_rs::cleaner::{clean, inspect};

let bytes = std::fs::read("draft.docx").unwrap();
let output = clean(&bytes, None).unwrap();
std::fs::write("draft.cleaned.docx", &output.bytes).unwrap();
println!("Chunks removed: {}", output.stats.metadata_chunks_removed);
```

## Feature Flags

| Feature  | Enables                    |
| -------- | -------------------------- |
| `python` | PyO3 extension module      |
| `node`   | napi-rs Node.js add-on     |
| `wasm`   | wasm-bindgen WASM bindings |

## 🔍 See Also: Core Logic & Reasoning

The core implementation works identically regardless of binding (Rust, Python, Node, WASM).

1. **`src/unicode.rs`** — **Layer A (Text)**
   - Text sweeps process string characters asynchronously against a static known list of Unicode categories (`STRIP_CODEPOINTS`, `EMOJI_GLUE`, etc.).
   - This operates at `O(1)` per-character space/time complexity via static pattern matching and binary search over codepoint slices.

2. **`src/image_meta.rs` & `src/container_meta.rs`** — **Metadata layer**
   - Media cleaners rely on byte boundary scanning or deterministic structural formats (e.g., zip streams for docx). We don't read entire payloads into graphics stacks, we do zero-allocation sub-slice byte patching where possible to ensure robust minimal modifications without risk of arbitrary-code execution on maliciously formed headers.

3. **`src/cleaner.rs`** — **Format Auto-Detection**
   - Implements a magic-byte sniffer prioritizing fast chunk detection. If it cannot identify a header (like a PNG `89 50 4E 47`), it falls back to parsing as UTF-8 plaintext.
