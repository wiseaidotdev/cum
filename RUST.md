# Rust Usage Guide: `cum-rs`

## Installation

```toml
[dependencies]
cum-rs = "0.2.0"
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

## Stochastic Enhancement (Layer B)

```rust
use cum_rs::stochastic::StochasticEnhancer;

// Create an enhancer with 70% substitution probability
let enhancer = StochasticEnhancer::new(0.7);

let output = enhancer.enhance("The chaos governs the universe.");
println!("Enhanced text: {}", output.text);
println!("Words substituted: {}", output.words_substituted);
```

## Image Metadata Stripping

```rust,no_run
use cum_rs::image_meta::clean_image;
use std::fs;

let png = fs::read("input.png").unwrap();
let cleaned = clean_image(&png).unwrap();
fs::write("output.png", &cleaned).unwrap();
```

## Unified API (auto-detect format)

```rust,no_run
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
