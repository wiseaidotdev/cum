# Unmark: Test Files

Drop or paste these files into the app at **http://127.0.0.1:3000/** to validate each watermark removal layer.

## Text files (paste into the textarea)

| File                          | Watermarks embedded                                                   | Expected result                                                       |
| ----------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `watermarked_zwsp.txt`        | **Zero-width spaces** (U+200B) between every word                     | All ZWSP stripped; word count in stats                                |
| `watermarked_bidi.txt`        | **BOM** (U+FEFF) + **bidirectional controls** LRE/PDF (U+202A/U+202C) | BOM and bidi chars stripped                                           |
| `watermarked_tags.txt`        | **Tag characters** U+E0001-U+E007F (spells "CLAUDE")                  | Tag block stripped                                                    |
| `watermarked_mixed.txt`       | ZWSP + BOM + bidi + word joiners + variation selectors + tags         | All carrier types detected and removed                                |
| `watermarked_confusables.txt` | **Cyrillic homoglyphs** substituted for Latin letters                 | Shown as "latin_confusable" findings (aggressive mode off by default) |

### How to paste

1. Open `watermarked_*.txt` in a text editor.
1. Select all (`Ctrl+A`) and copy (`Ctrl+C`).
1. Click the **Text** tab in Unmark and paste.
1. The output panel updates automatically after ~500 ms.

## Markdown file (paste as text or drop as file)

| File                         | Metadata                                                                    | Expected result                     |
| ---------------------------- | --------------------------------------------------------------------------- | ----------------------------------- |
| `watermarked_frontmatter.md` | YAML front-matter keys `ai-generated`, `model`, `generator`, `watermark-id` | AI keys stripped; `title` preserved |

## Binary files (drop into the File tab)

| File              | Metadata                                                              | Expected result                                 |
| ----------------- | --------------------------------------------------------------------- | ----------------------------------------------- |
| `watermarked.png` | `iTXt AI:Provider=Anthropic`, `tEXt Software=Anthropic Claude Opus 5` | Both chunks stripped; valid 1x1 red PNG remains |

### How to use binary files

1. Click the **File** tab in Unmark.
1. Drag `watermarked.png` (or any file) onto the drop zone,  
   or click **Browse files** and select it.
1. The cleaned file appears immediately in the output panel.
1. Click **Download** to save the clean version.
