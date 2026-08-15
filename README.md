<div align="center">

<img src="https://raw.githubusercontent.com/wiseaidotdev/cum/main/assets/logo.webp" alt="cum-rs logo" width="220"/>

# CUM

[![Crates.io](https://img.shields.io/crates/v/cum-rs.svg)](https://crates.io/crates/cum-rs)
[![Docs.rs](https://docs.rs/cum-rs/badge.svg)](https://docs.rs/cum-rs)
[![npm](https://img.shields.io/npm/v/cum-rs.svg)](https://www.npmjs.com/package/cum-rs)
[![PyPI](https://img.shields.io/pypi/v/cum-rs.svg)](https://pypi.org/project/cum-rs)
[![CI](https://github.com/wiseaidotdev/cum/actions/workflows/rust.yml/badge.svg)](https://github.com/wiseaidotdev/cum/actions/workflows/rust.yml)
[![Clippy](https://github.com/wiseaidotdev/cum/actions/workflows/clippy.yml/badge.svg)](https://github.com/wiseaidotdev/cum/actions/workflows/clippy.yml)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.89+-orange.svg)](https://blog.rust-lang.org/)

> **Claude Unmarking Machine**: a multilanguage Rust crate that removes
> AI-provider watermarks from text, images, and documents.
> Works regardless of provider (Claude, OpenAI, Gemini, Grok, open-LLM).
> All processing is **100% local**: no data leaves your machine.

![crab dancing](https://raw.githubusercontent.com/wiseaidotdev/cum/main/assets/crabby-dance.gif)

_The `cum` binary, cheerfully evicting zero-width gremlins from your prose._

</div>

## 🤔 What is happening here, exactly?

So you copy-pasted some text from an AI. Totally normal. You are not doing anything illegal. Probably.

The bad news: every major LLM provider stuffs your output full of invisible Unicode graffiti so they can identify their own generation later. It is like spray-painting "CLAUDE WAS HERE" on every wall, except the paint is literally invisible and you cannot see it without specialized equipment.

The good news: we have the specialized equipment. And it is written in Rust, so it is blazingly fast.

| Layer              | What lurks in the shadows                                                                                     | What we do about it                                                   |
| ------------------ | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| **A: Unicode**     | ZWSP, bidi controls, tag chars, variation selectors, private-use codepoints, basically a Unicode horror movie | Deterministic, lossless exorcism 🧹                                   |
| **File: Metadata** | C2PA manifests, EXIF, XMP, document properties, the digital equivalent of a tracking ankle bracelet           | Stripped from PNG, JPEG, WebP, SVG, PDF, DOCX, ODT, HTML, Markdown    |
| **B: Statistical** | Token-sampling watermarks (SynthID-Text, KGW), watermarks baked into the actual word choices                  | Best-effort via stochastic synonym replacement: `enhance` command / `enhance_text` API |

> **Fun fact:** some of those invisible characters are technically in the Unicode "Tag" block, which was originally designed for plane tickets in 1997 and then deprecated. AI providers found a new use for them. The Unicode Consortium is presumably very proud.

## 🖥️ CLI: for the terminal warriors

Install the `cum` binary. Yes, that is the name. Yes, the authors are aware. Yes, it compiles clean:

```sh
cargo install cum-rs --features rust-binary
```

Then run:

```sh
# Clean a Markdown file. The AI left crumbs everywhere.
cum clean report.md

# Clean a PNG: yes, even images can be watermarked now. We live in a society.
cum clean logo.png --output logo_clean.png

# Inspect your text for hidden nonsense, formatted as JSON for maximum nerd points
cum inspect --json article.txt

# Pipe from stdin like a true Unix philosopher
cat suspicious.txt | cum clean --stdin

# The aggressive mode: also replaces Cyrillic А with Latin A (sneaky!)
cum clean --aggressive sneaky_essay.txt
```

See **[CLI.md](CLI.md)** for the full command reference. It has tables and everything.

## 🦀 Rust: the fast one

Available on [crates.io](https://crates.io/crates/cum-rs). Because of course it is. Full API docs: **[RUST.md](RUST.md)**.

| Feature       | Description                                                                                     |
| ------------- | ----------------------------------------------------------------------------------------------- |
| _(default)_   | Pure-Rust core: `clean`, `inspect`, all media formats. Zero drama.                              |
| `cli`         | Clap CLI module required for the binary. Comes with an ASCII banner, because we have standards. |
| `rust-binary` | Enables the `cum` binary. Ship it.                                                              |
| `python`      | Python extension via PyO3. For the snake people. 🐍                                             |
| `node`        | Node.js native add-on via napi-rs. For the `node_modules` enjoyers. 🟩                          |
| `wasm`        | WASM bindings. Run watermark removal in the browser. Why? Because we can.                       |

## ⚡ Quick Start

```rust
use cum_rs::cleaner::clean;
use cum_rs::types::MediaHint;

// "Hello​ world﻿!": looks innocent, contains a ZWSP and a BOM. Rude.
let dirty = "Hello\u{200B} world\u{FEFF}!";
let output = clean(dirty.as_bytes(), Some(MediaHint::Text)).unwrap();

// Now it is just "Hello world!" like a normal person wrote it
assert_eq!(String::from_utf8(output.bytes).unwrap(), "Hello world!");
assert_eq!(output.stats.removed_count, 2); // Two gremlins evicted. You're welcome.
```

## 🌐 WASM

Because if you are going to remove watermarks, you might as well do it at the speed of JavaScript. (Do not worry, the Rust core still does the actual work.) See **[WASM.md](WASM.md)** and the live demo: [`examples/unmark/`](examples/unmark/).

## 🐍 Python

```python
import cum_rs

result = cum_rs.clean_text("Hello\u200b world\ufeff!")
print(result.cleaned)        # "Hello world!"
print(result.removed_count)  # 2
# The AI's fingerprints have been thoroughly wiped. You were never here.
```

See **[PYTHON.md](PYTHON.md)** for the full binding reference.

## 🟩 Node.js

```javascript
const { cleanText } = require("cum-rs");

const result = cleanText("Hello\u200b world\ufeff!");
console.log(result.cleaned); // "Hello world!"
console.log(result.removedCount); // 2
// node_modules is 9000 packages deep but THIS one actually does something useful
```

See **[NODE.md](NODE.md)** for the full binding reference.

## 🎲 Stochastic Enhancer

CUM includes a best-effort countermeasure against Layer B statistical watermarks (SynthID, KGW) by stochastically replacing eligible words with semantically equivalent synonyms. This modifies the raw byte pairs chosen by the LLM's token-sampler, disrupting the periodic watermark signal.

```rust
use cum_rs::stochastic::StochasticEnhancer;

let enhancer = StochasticEnhancer::new(0.5); // 50% substitution chance
let output = enhancer.enhance("The chaos governs the universe.");
println!("Substituted {} words", output.words_substituted);
println!("{}", output.text);
```

The synonyms table relies on a curated perfect-hash map combined with the host's `/usr/share/dict/` system wordlist.

## 🚨 Disclaimer _(the responsible adult part)_

**Layer A** (Unicode scrubbing) and **file metadata** stripping are fully deterministic and lossless: every modification is logged in `stats`. You can see exactly what changed.

**Layer B** (statistical watermarks) lives inside the actual word choices. No tool can guarantee removal. The only real fix is to rewrite the content in your own words. Think of Layer B as the AI watermarking the _vibes_ of the text, not just the characters.

This crate is for **content you own**: research, privacy hygiene, and understanding what AI providers are doing to your outputs. Read [`ETHICS.md`](ETHICS.md) before doing anything exciting.

## 📄 License

MIT. Do what you want. Just do not be evil about it.
