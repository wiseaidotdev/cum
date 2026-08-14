# `cum` Command Line Interface 🧹

The standalone `cum` binary exposes all watermark removal and inspection
capabilities from the `cum-rs` library directly in your terminal.

## 📦 Installation

```sh
cargo install cum-rs --features rust-binary
```

The resulting binary is named `cum`.

## 🛠 Subcommands

### `cum clean`: Remove watermarks

```
cum clean [OPTIONS] [FILE]
```

Cleans a file **or** inline text. Output goes to stdout by default; redirect
with `--output` to write a file.

| Argument                | Default | Description                                                                                         |
| ----------------------- | ------- | --------------------------------------------------------------------------------------------------- |
| `[FILE]`                | -       | Path to file (PNG, JPEG, WebP, SVG, PDF, DOCX, ODT, HTML, Markdown, plain text).                    |
| `-t`, `--text <TEXT>`   | -       | Inline text to clean (alternative to a file path).                                                  |
| `--stdin`               | -       | Read bytes from stdin.                                                                              |
| `-o`, `--output <OUT>`  | stdout  | Write cleaned output to this path.                                                                  |
| `-a`, `--aggressive`    | `true`  | Also replace Cyrillic / fullwidth-Latin confusable letters.                                         |
| `-m`, `--media <MEDIA>` | auto    | Force a media type: `text`, `png`, `jpeg`, `webp`, `svg`, `pdf`, `docx`, `odt`, `html`, `markdown`. |
| `-j`, `--json`          | off     | Print stats as JSON to stderr.                                                                      |
| `-q`, `--quiet`         | off     | Suppress progress output.                                                                           |

#### Examples

```bash
# Clean a text file: output to stdout
cum clean report.md

# Clean a PNG in-place
cum clean logo.png --output logo_clean.png

# Clean inline text
cum clean --text "Hello​ world"   # contains ZWSP

# Pipe from another command
cat suspicious.txt | cum clean --stdin

# JSON stats while writing cleaned file
cum clean --json -o clean.txt dirty.txt
```

### `cum inspect` - Detect watermarks

```
cum inspect [OPTIONS] [FILE]
```

Inspects a file or text for watermark carriers **without modifying it**.
Prints a per-codepoint table to stdout.

| Argument                | Default | Description                                      |
| ----------------------- | ------- | ------------------------------------------------ |
| `[FILE]`                | -       | File to inspect.                                 |
| `-t`, `--text <TEXT>`   | -       | Inline text.                                     |
| `--stdin`               | -       | Read from stdin.                                 |
| `-a`, `--aggressive`    | `true`  | Include Cyrillic / fullwidth confusable matches. |
| `-m`, `--media <MEDIA>` | auto    | Force media type.                                |
| `-j`, `--json`          | off     | Emit findings as JSON.                           |

#### Examples

```bash
# Inspect a text file
cum inspect article.txt

# Machine-readable JSON output
cum inspect --json image.png

# Inspect inline text
cum inspect --text "C​laude"   # ZWSP between C and l

# Pipe a downloaded webpage
curl -s https://example.com | cum inspect --stdin --media html
```

## Supported Formats

| Extension       | Detection            | Layer                 |
| --------------- | -------------------- | --------------------- |
| `.txt`, `.md`   | extension            | Unicode Layer A       |
| `.html`, `.htm` | extension / magic    | Unicode + HTML meta   |
| `.png`          | magic bytes          | PNG chunk strip       |
| `.jpg`, `.jpeg` | magic bytes          | JPEG APP1/APP11/APP13 |
| `.webp`         | magic bytes          | RIFF EXIF/XMP         |
| `.svg`          | magic bytes          | XML metadata          |
| `.pdf`          | magic bytes          | XMP/Info byte-scan    |
| `.docx`         | magic bytes (ZIP PK) | docProps strip        |
| `.odt`          | magic bytes (ZIP PK) | meta.xml strip        |

## What Gets Removed

| Watermark class                  | Examples                                              | Action               |
| -------------------------------- | ----------------------------------------------------- | -------------------- |
| Invisible controls               | ZWSP (U+200B), BOM (U+FEFF), WJ (U+2060)              | Strip                |
| Bidirectional format controls    | LRE, RLE, PDF, LRI, RLI, FSI, PDI                     | Strip                |
| Tag characters                   | U+E0001-U+E007F                                       | Strip                |
| Variation selectors              | FE00-FE0F, E0100-E01EF (non-emoji)                    | Strip                |
| Private-use                      | E000-F8FF, F0000-10FFFD                               | Strip                |
| Space homoglyphs                 | En-Quad, Hair Space, Narrow NBSP, ...                 | Replace → U+0020     |
| Cyrillic / fullwidth confusables | Cyr А→A, FF21→A, ...                                  | Replace (aggressive) |
| PNG metadata chunks              | iTXt, tEXt, zTXt, eXIf, C2PA (caBX/JUMB)              | Strip chunk          |
| JPEG metadata segments           | APP1 (EXIF/XMP), APP11 (JUMBF), APP13 (IPTC)          | Strip segment        |
| WebP RIFF chunks                 | EXIF, XMP, C2PA                                       | Strip chunk          |
| SVG embedded metadata            | `<metadata>`, `<x:xmpmeta>`, `data-ai-*`              | Strip                |
| PDF metadata                     | XMP xpacket, `/Author`, `/Creator`, `/Producer`       | Byte-scan strip      |
| DOCX/ODT metadata                | `docProps/core.xml`, `docProps/app.xml`, `customXml/` | ZIP repack           |
| HTML metadata                    | `<meta name="generator">`, `data-ai-*`, JSON-LD C2PA  | Strip                |
| Markdown front-matter            | `ai-generated`, `model`, `generator`, `watermark-id`  | Strip keys           |
