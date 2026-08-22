// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Top-level application component.

use crate::components::controls_panel::ControlsPanel;
use crate::components::header::Header;
use crate::components::input_panel::InputPanel;
use crate::components::output_panel::OutputPanel;
use crate::types::{AppLanguage, CleanResult, CleanStats, MediaKind, StochasticConfig};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use cum_rs::cleaner::clean;
use cum_rs::stochastic::{LanguageHint, StochasticEnhancer};
use cum_rs::types::MediaHint;
use cum_rs::unicode::{CleanOpts, clean_text};
use web_sys::MouseEvent;
use yew::prelude::*;

/// Returns the [`StochasticConfig`] default: disabled, 50 %, Auto language, Layer A normalize ON.
fn default_stochastic_config() -> StochasticConfig {
    StochasticConfig {
        enabled: false,
        probability_pct: 50,
        language: AppLanguage::Auto,
        normalize_punctuation: true,
    }
}

/// Maps the UI [`AppLanguage`] to the crate [`LanguageHint`].
fn to_lang_hint(lang: &AppLanguage) -> LanguageHint {
    match lang {
        AppLanguage::Auto => LanguageHint::Auto,
        AppLanguage::English => LanguageHint::English,
        AppLanguage::Spanish => LanguageHint::Spanish,
        AppLanguage::French => LanguageHint::French,
        AppLanguage::German => LanguageHint::German,
        AppLanguage::Arabic => LanguageHint::Arabic,
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let text = use_state(String::new);
    let result = use_state(|| None::<CleanResult>);
    let active_tab = use_state(|| "text".to_string());
    let file_bytes = use_state(|| None::<Vec<u8>>);
    let file_name = use_state(|| None::<String>);
    let stochastic = use_state(default_stochastic_config);

    let debounce = use_mut_ref(|| Option::<gloo_timers::callback::Timeout>::None);

    let on_text_change = {
        let text_state = text.clone();
        let result_state = result.clone();
        let stochastic = stochastic.clone();
        let debounce = debounce.clone();
        Callback::from(move |s: String| {
            text_state.set(s.clone());
            *debounce.borrow_mut() = None;

            if s.trim().is_empty() {
                result_state.set(None);
                return;
            }

            let rs = result_state.clone();
            let sc = stochastic.clone();
            let typed = s.clone();
            let t = gloo_timers::callback::Timeout::new(400, move || {
                rs.set(Some(run_clean_text(&typed, &sc)));
            });
            *debounce.borrow_mut() = Some(t);
        })
    };

    let on_clear: Callback<MouseEvent> = {
        let text_state = text.clone();
        let result_state = result.clone();
        Callback::from(move |_: MouseEvent| {
            text_state.set(String::new());
            result_state.set(None);
        })
    };

    let on_stochastic_change = {
        let stochastic_state = stochastic.clone();
        let text_val = text.clone();
        let result_state = result.clone();
        Callback::from(move |cfg: StochasticConfig| {
            stochastic_state.set(cfg.clone());
            let current = (*text_val).clone();
            if !current.trim().is_empty() {
                result_state.set(Some(run_clean_text(&current, &cfg)));
            }
        })
    };

    let on_tab_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: String| active_tab.set(tab))
    };

    let on_file = {
        let result_state = result.clone();
        let name_state = file_name.clone();
        let bytes_state = file_bytes.clone();
        Callback::from(move |files: Vec<(String, Vec<u8>)>| {
            if let Some((name, bytes)) = files.into_iter().next() {
                result_state.set(Some(run_clean_bytes(Some(&bytes), Some(&name))));
                name_state.set(Some(name));
                bytes_state.set(Some(bytes));
            }
        })
    };

    html! {
        <div class="flex flex-col h-screen bg-um-bg font-sans">
            <Header />
            <main
                class="flex-1 overflow-y-auto lg:overflow-hidden flex flex-col lg:flex-row gap-3 p-3 md:p-4 min-h-0"
            >
                <div class="flex flex-col gap-3 w-full lg:w-5/12 lg:max-w-md shrink-0 lg:min-h-0">
                    <div class="flex-none lg:flex-[1] flex flex-col lg:min-h-0 min-h-[350px]">
                        <InputPanel
                            text_value={(*text).clone()}
                            on_text_change={on_text_change}
                            on_file={on_file}
                            active_tab={(*active_tab).clone()}
                            on_tab_change={on_tab_change}
                            on_clear={on_clear}
                        />
                    </div>
                    <div
                        class="flex-none lg:flex-[1] flex flex-col lg:min-h-0 overflow-y-auto custom-scrollbar rounded-xl"
                    >
                        <ControlsPanel
                            config={(*stochastic).clone()}
                            on_change={on_stochastic_change}
                        />
                    </div>
                </div>
                <div class="flex-1 min-h-[400px] lg:min-h-0 flex flex-col">
                    <OutputPanel
                        result={(*result).clone()}
                        stochastic_enabled={(*stochastic).enabled}
                        loading=false
                    />
                </div>
            </main>
        </div>
    }
}

/// Runs the Layer-A clean followed by an optional stochastic enhancement pass.
///
/// The two passes are chained: enhancement runs on the _already cleaned_ text
/// so the synonym substitution never accidentally re-introduces watermark
/// carriers.
///
/// # Time Complexity
///
/// O(n) for cleaning + O(t) for enhancement, where n is the character count
/// and t is the token count.
///
/// # Space Complexity
///
/// O(n) for the output buffer.
fn run_clean_text(text: &str, stochastic: &StochasticConfig) -> CleanResult {
    let opts = CleanOpts {
        aggressive_confusables: true,
        normalize_punctuation: stochastic.normalize_punctuation,
        ..CleanOpts::safe()
    };
    match clean_text(text, &opts) {
        Ok((cleaned, raw)) => {
            let mut final_text = cleaned;
            let mut summary = raw.summary;
            let mut replaced_count = raw.replaced_count;

            if stochastic.enabled {
                let lang_hint = to_lang_hint(&stochastic.language);
                let prob = stochastic.probability_pct as f64 / 100.0;
                let enhancer = StochasticEnhancer::with_language_and_probability(lang_hint, prob);
                let out = enhancer.enhance(&final_text);
                final_text = out.text;
                if out.words_substituted > 0 {
                    replaced_count += out.words_substituted;
                    summary.push(format!("layer_b_synonyms: {}", out.words_substituted));
                }
            }

            CleanResult {
                bytes: final_text.into_bytes(),
                stats: CleanStats {
                    removed_count: raw.removed_count,
                    replaced_count,
                    metadata_chunks_removed: raw.metadata_chunks_removed,
                    summary,
                },
                kind: MediaKind::Text,
                image_data_url: None,
            }
        }
        Err(e) => CleanResult {
            bytes: format!("Error: {e}").into_bytes(),
            stats: CleanStats::default(),
            kind: MediaKind::Text,
            image_data_url: None,
        },
    }
}

/// Runs the format-auto-detected clean for binary inputs (images, documents).
///
/// Stochastic enhancement is not applied to binary inputs; it is text-only.
///
/// # Time Complexity
///
/// O(n) where n is the byte length of the input.
///
/// # Space Complexity
///
/// O(n).
fn run_clean_bytes(bytes: Option<&[u8]>, name: Option<&str>) -> CleanResult {
    let Some(bytes) = bytes else {
        return CleanResult {
            bytes: b"No file loaded.".to_vec(),
            stats: CleanStats::default(),
            kind: MediaKind::Text,
            image_data_url: None,
        };
    };

    let hint = file_hint(name.unwrap_or(""));
    let mime = hint_mime(&hint);

    match clean(bytes, Some(hint.clone())) {
        Ok(out) => {
            let image_data_url = match &hint {
                MediaHint::Png | MediaHint::Jpeg | MediaHint::Webp | MediaHint::Svg => {
                    Some(format!("data:{mime};base64,{}", B64.encode(&out.bytes)))
                }
                _ => None,
            };
            CleanResult {
                bytes: out.bytes,
                stats: CleanStats {
                    removed_count: out.stats.removed_count,
                    replaced_count: out.stats.replaced_count,
                    metadata_chunks_removed: out.stats.metadata_chunks_removed,
                    summary: out.stats.summary,
                },
                kind: hint_kind(&hint, name.unwrap_or("file")),
                image_data_url,
            }
        }
        Err(e) => CleanResult {
            bytes: format!("Error: {e}").into_bytes(),
            stats: CleanStats::default(),
            kind: MediaKind::Text,
            image_data_url: None,
        },
    }
}

/// Maps a file-name extension to the appropriate [`MediaHint`].
fn file_hint(name: &str) -> MediaHint {
    let l = name.to_lowercase();
    if l.ends_with(".png") {
        MediaHint::Png
    } else if l.ends_with(".jpg") || l.ends_with(".jpeg") {
        MediaHint::Jpeg
    } else if l.ends_with(".webp") {
        MediaHint::Webp
    } else if l.ends_with(".svg") {
        MediaHint::Svg
    } else if l.ends_with(".pdf") {
        MediaHint::Pdf
    } else if l.ends_with(".docx") {
        MediaHint::Docx
    } else if l.ends_with(".odt") {
        MediaHint::Odt
    } else if l.ends_with(".html") || l.ends_with(".htm") {
        MediaHint::Html
    } else if l.ends_with(".md") || l.ends_with(".markdown") {
        MediaHint::Markdown
    } else {
        MediaHint::Text
    }
}

/// Maps a [`MediaHint`] to its primary MIME type string.
fn hint_mime(hint: &MediaHint) -> &'static str {
    match hint {
        MediaHint::Png => "image/png",
        MediaHint::Jpeg => "image/jpeg",
        MediaHint::Webp => "image/webp",
        MediaHint::Svg => "image/svg+xml",
        MediaHint::Pdf => "application/pdf",
        MediaHint::Docx => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        MediaHint::Odt => "application/vnd.oasis.opendocument.text",
        MediaHint::Html => "text/html",
        MediaHint::Markdown | MediaHint::Text => "text/plain",
    }
}

/// Classifies a [`MediaHint`] into a [`MediaKind`] for display purposes.
fn hint_kind(hint: &MediaHint, name: &str) -> MediaKind {
    match hint {
        MediaHint::Png => MediaKind::Image("image/png".into()),
        MediaHint::Jpeg => MediaKind::Image("image/jpeg".into()),
        MediaHint::Webp => MediaKind::Image("image/webp".into()),
        MediaHint::Svg => MediaKind::Image("image/svg+xml".into()),
        MediaHint::Text | MediaHint::Markdown => MediaKind::Text,
        _ => MediaKind::Document(name.to_string()),
    }
}
