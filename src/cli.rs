// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # CLI layer
//!
//! Defines the top-level [`Cli`] struct and every subcommand that the `cum`
//! binary exposes.  All heavy lifting is delegated to the library crate so
//! this module stays thin.

use crate::cleaner::{clean, inspect};
use crate::stochastic::StochasticEnhancer;
use crate::types::MediaHint;
use crate::unicode::{CleanOpts, InspectOpts, clean_text, inspect_text};
use anyhow::{Context, Result, bail};
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Magenta.on_default() | Effects::BOLD)
        .usage(AnsiColor::Magenta.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Green.on_default())
}

/// 🧹 cum-rs: Claude Unmarking Machine
///
/// Remove AI-provider watermarks from text, images, and documents.
/// All processing is 100% local: no data is sent anywhere.
///
/// Repository: <https://github.com/wiseaidotdev/cum>
#[derive(Debug, Parser)]
#[command(
    name = "cum",
    author = "Mahmoud Harmouch <oss@wiseai.dev>",
    version = env!("CARGO_PKG_VERSION"),
    propagate_version = true,
    arg_required_else_help = true,
    styles = styles(),
    help_template = r#"{before-help}{about}

{usage-heading} {usage}

{all-args}{after-help}

AUTHORS:
    {author}
"#,
    about = r#"
 ██████╗ ██╗   ██╗███╗   ███╗
██╔════╝ ██║   ██║████╗ ████║
██║      ██║   ██║██╔████╔██║
██║      ██║   ██║██║╚██╔╝██║
╚██████╗ ╚██████╔╝██║ ╚═╝ ██║
 ╚═════╝  ╚═════╝ ╚═╝     ╚═╝
 Claude Unmarking Machine 
=========================

Remove AI-provider watermarks from text, images, and documents safely and completely offline.

FEATURES:
  - Text format checking: Strip Layer A Unicode carriers (ZWSP, Tags, Variations).
  - Confusable patching: --aggressive swaps Cyrillic / fullwidth homoglyphs back to Latin.
  - Image sweeping: Remove hidden C2PA or invisible EXIF tracking.
  - Document sweeping: Scrub hidden markers in PDF, DOCX, ODT.
  - Stochastic enhancement: Best-effort Layer B synonym substitution to defeat token-sampling watermarks.
  - Media auto-detection: Format guessed from magic bytes automatically.
  - Output formatting: --json output for programmatic scraping.

USAGE:
  cum [OPTIONS] <COMMAND>

EXAMPLES:
  - Clean inline text using --text (-t):
    cum clean --text "Hello world"

  - Strip watermarks from an image to an output file:
    cum clean profile.jpeg --output profile_clean.jpeg

  - Inspect metadata without modifying:
    cum inspect suspected.pdf

  - Aggressively fix spacing/confusables over stdin:
    cat prompt.txt | cum clean --stdin -a

  - Apply stochastic synonym replacement at 70% probability:
    cum enhance --text "The chaos governs the universe" --probability 0.7

  - Enhance a text file and save the result:
    cum enhance report.txt --probability 0.5 --output report_enhanced.txt

For more detail, check CLI.md or https://github.com/wiseaidotdev/cum
"#
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output results as JSON (machine-readable).
    #[arg(long, short = 'j', global = true)]
    pub json: bool,

    /// Suppress all output except errors.
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Clean a file or text, removing all watermark carriers.
    #[command(visible_alias = "c")]
    Clean {
        /// Path to the file to clean.  Omit to read from `--text` or stdin.
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Inline text to clean (alternative to supplying a file).
        #[arg(long, short = 't', value_name = "TEXT", conflicts_with = "file")]
        text: Option<String>,

        /// Read input from stdin.
        #[arg(long, conflicts_with_all = &["file", "text"])]
        stdin: bool,

        /// Write cleaned bytes to this path instead of stdout.
        #[arg(long, short = 'o', value_name = "OUT")]
        output: Option<PathBuf>,

        /// Also replace Cyrillic / fullwidth-Latin confusable letters.
        #[arg(long, short = 'a', default_value_t = true)]
        aggressive: bool,

        /// Force treat input as this media type (skipping auto-detection).
        #[arg(long, short = 'm', value_name = "MEDIA", value_enum)]
        media: Option<MediaArg>,
    },

    /// Inspect a file or text for watermark carriers without modifying it.
    #[command(visible_alias = "i")]
    Inspect {
        /// Path to the file to inspect.  Omit to pipe from `--text` or stdin.
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Inline text to inspect.
        #[arg(long, short = 't', value_name = "TEXT", conflicts_with = "file")]
        text: Option<String>,

        /// Read input from stdin.
        #[arg(long, conflicts_with_all = &["file", "text"])]
        stdin: bool,

        /// Also check Cyrillic / fullwidth-Latin confusable letters.
        #[arg(long, short = 'a', default_value_t = true)]
        aggressive: bool,

        /// Force treat input as this media type.
        #[arg(long, short = 'm', value_name = "MEDIA", value_enum)]
        media: Option<MediaArg>,
    },

    /// Apply stochastic synonym substitution to defeat Layer B statistical watermarks.
    ///
    /// Each non-stop word is replaced with a synonym with the given probability,
    /// using a curated PHF table and the Linux system dictionary as fallback.
    #[command(visible_alias = "e")]
    Enhance {
        /// Path to the plain-text file to enhance.  Omit to use `--text` or stdin.
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Inline text to enhance (alternative to supplying a file).
        #[arg(long, short = 't', value_name = "TEXT", conflicts_with = "file")]
        text: Option<String>,

        /// Read plain-text input from stdin.
        #[arg(long, conflicts_with_all = &["file", "text"])]
        stdin: bool,

        /// Write the enhanced output to this path instead of stdout.
        #[arg(long, short = 'o', value_name = "OUT")]
        output: Option<PathBuf>,

        /// Per-word synonym-substitution probability in the range `[0.0, 1.0]`.
        ///
        /// 0.0 disables all substitution; 1.0 replaces every eligible word.
        #[arg(long, short = 'p', value_name = "PROB", default_value_t = 0.5)]
        probability: f64,
    },

    /// Print the crate version.
    #[command(hide = true)]
    Version,
}

/// Explicit media type override (skips magic-byte auto-detection).
#[derive(Debug, Clone, ValueEnum)]
pub enum MediaArg {
    Text,
    Png,
    Jpeg,
    Webp,
    Svg,
    Pdf,
    Docx,
    Odt,
    Html,
    Markdown,
}

impl From<MediaArg> for MediaHint {
    fn from(a: MediaArg) -> Self {
        match a {
            MediaArg::Text => MediaHint::Text,
            MediaArg::Png => MediaHint::Png,
            MediaArg::Jpeg => MediaHint::Jpeg,
            MediaArg::Webp => MediaHint::Webp,
            MediaArg::Svg => MediaHint::Svg,
            MediaArg::Pdf => MediaHint::Pdf,
            MediaArg::Docx => MediaHint::Docx,
            MediaArg::Odt => MediaHint::Odt,
            MediaArg::Html => MediaHint::Html,
            MediaArg::Markdown => MediaHint::Markdown,
        }
    }
}

/// Entry-point called from `src/bin/main.rs`.
pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
        }

        Command::Clean {
            file,
            text,
            stdin,
            output,
            aggressive,
            media,
        } => {
            run_clean(
                file, text, stdin, output, aggressive, media, cli.json, cli.quiet,
            )?;
        }

        Command::Inspect {
            file,
            text,
            stdin,
            aggressive,
            media,
        } => {
            run_inspect(file, text, stdin, aggressive, media, cli.json)?;
        }

        Command::Enhance {
            file,
            text,
            stdin,
            output,
            probability,
        } => {
            run_enhance(file, text, stdin, output, probability, cli.json, cli.quiet)?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_clean(
    file: Option<PathBuf>,
    text: Option<String>,
    stdin: bool,
    output: Option<PathBuf>,
    aggressive: bool,
    media: Option<MediaArg>,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let (bytes, is_text_mode) = resolve_input(file.as_deref(), text.as_deref(), stdin)?;

    let (cleaned_bytes, removed, replaced) = if is_text_mode {
        let s = std::str::from_utf8(&bytes).context("input is not valid UTF-8")?;
        let opts = CleanOpts {
            aggressive_confusables: aggressive,
            ..CleanOpts::safe()
        };
        let (clean, stats) = clean_text(s, &opts)?;
        (
            clean.into_bytes(),
            stats.removed_count,
            stats.replaced_count,
        )
    } else {
        let hint = media.map(MediaHint::from);
        let out = clean(&bytes, hint)?;
        let r = out.stats.removed_count;
        let rp = out.stats.replaced_count;
        (out.bytes, r, rp)
    };

    if let Some(path) = output {
        std::fs::write(&path, &cleaned_bytes)
            .with_context(|| format!("writing to {}", path.display()))?;
        if !quiet {
            eprintln!(
                "✅  Wrote cleaned output → {} ({} stripped, {} replaced)",
                path.display(),
                removed,
                replaced
            );
        }
    } else {
        if is_text_mode {
            print!("{}", String::from_utf8_lossy(&cleaned_bytes));
        } else {
            use std::io::Write;
            std::io::stdout().write_all(&cleaned_bytes)?;
        }
        if !quiet && json {
            eprintln!(
                "{}",
                serde_json::json!({
                    "removed_count": removed,
                    "replaced_count": replaced,
                })
            );
        } else if !quiet {
            eprintln!("🧹  {} stripped, {} replaced", removed, replaced);
        }
    }
    Ok(())
}

fn run_inspect(
    file: Option<PathBuf>,
    text: Option<String>,
    stdin: bool,
    aggressive: bool,
    media: Option<MediaArg>,
    json: bool,
) -> Result<()> {
    let (bytes, is_text_mode) = resolve_input(file.as_deref(), text.as_deref(), stdin)?;

    if is_text_mode {
        let s = std::str::from_utf8(&bytes).context("input is not valid UTF-8")?;
        let opts = InspectOpts {
            aggressive_confusables: aggressive,
            ..InspectOpts::default()
        };
        let report = inspect_text(s, &opts)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            let total = report.hits.iter().map(|h| h.count).sum::<usize>();
            if total == 0 {
                println!("✅  No watermarks detected.");
            } else {
                println!("⚠️   {} suspicious codepoint instance(s) found:", total);
                for hit in &report.hits {
                    println!(
                        "  U+{:04X}  {:?}  ×{}  ({:?})",
                        hit.codepoint, hit.kind, hit.count, hit.confidence
                    );
                }
            }
        }
    } else {
        let hint = media.map(MediaHint::from);
        let out = inspect(&bytes, hint)?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "format": format!("{:?}", out.format),
                    "meta_findings": out.meta_findings,
                }))?
            );
        } else {
            let findings = &out.meta_findings;
            if findings.is_empty() {
                println!("✅  No metadata watermarks found.");
            } else {
                println!("⚠️   {} finding(s):", findings.len());
                for f in findings {
                    println!("  [{:?}] {}", f.confidence, f.description);
                }
            }
        }
    }
    Ok(())
}

/// Applies stochastic synonym substitution to plain-text input.
///
/// # Complexity
/// - Time: O(t) where t is the number of whitespace-separated tokens.
/// - Space: O(t).
#[allow(clippy::too_many_arguments)]
fn run_enhance(
    file: Option<PathBuf>,
    text: Option<String>,
    stdin: bool,
    output: Option<PathBuf>,
    probability: f64,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let (bytes, _) = resolve_input(file.as_deref(), text.as_deref(), stdin)?;
    let input = std::str::from_utf8(&bytes).context("input is not valid UTF-8")?;
    let enhancer = StochasticEnhancer::new(probability);
    let out = enhancer.enhance(input);

    if let Some(path) = output {
        std::fs::write(&path, out.text.as_bytes())
            .with_context(|| format!("writing to {}", path.display()))?;
        if !quiet {
            eprintln!(
                "✨  Wrote enhanced output → {} ({} words substituted, p={:.2})",
                path.display(),
                out.words_substituted,
                out.probability,
            );
        }
    } else {
        print!("{}", out.text);
        if !quiet && json {
            eprintln!(
                "{}",
                serde_json::json!({
                    "words_substituted": out.words_substituted,
                    "probability": out.probability,
                })
            );
        } else if !quiet {
            eprintln!(
                "✨  {} word(s) substituted (p={:.2})",
                out.words_substituted, out.probability,
            );
        }
    }
    Ok(())
}

/// Returns `(bytes, is_text_mode)`.
/// `is_text_mode` is true when the source is a raw `--text` string or stdin
/// without an explicit `--media` binary type.
fn resolve_input(
    file: Option<&std::path::Path>,
    text: Option<&str>,
    stdin: bool,
) -> Result<(Vec<u8>, bool)> {
    if let Some(t) = text {
        return Ok((t.as_bytes().to_vec(), true));
    }
    if stdin {
        use std::io::Read;
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        return Ok((buf, false));
    }
    if let Some(path) = file {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let is_text = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("txt" | "md" | "markdown" | "html" | "htm")
        );
        return Ok((bytes, is_text));
    }
    bail!("Provide a FILE, --text TEXT, or --stdin")
}
