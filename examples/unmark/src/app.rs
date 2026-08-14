// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Top-level application component.

use crate::components::header::Header;
use crate::components::input_panel::InputPanel;
use crate::components::output_panel::OutputPanel;
use crate::types::{CleanResult, CleanStats, MediaKind};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use cum_rs::cleaner::clean;
use cum_rs::types::MediaHint;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let text = use_state(String::new);
    let result = use_state(|| None::<CleanResult>);
    let active_tab = use_state(|| "text".to_string());
    let file_bytes = use_state(|| None::<Vec<u8>>);
    let file_name = use_state(|| None::<String>);

    let debounce = use_mut_ref(|| Option::<gloo_timers::callback::Timeout>::None);

    let on_text_change = {
        let text_state = text.clone();
        let result_state = result.clone();
        let debounce = debounce.clone();
        Callback::from(move |s: String| {
            text_state.set(s.clone());

            *debounce.borrow_mut() = None;

            if s.trim().is_empty() {
                return;
            }

            let rs = result_state.clone();
            let typed = s.clone();
            let t = gloo_timers::callback::Timeout::new(500, move || {
                rs.set(Some(run_clean_text(&typed)));
            });
            *debounce.borrow_mut() = Some(t);
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
        <div class="flex flex-col h-screen overflow-hidden bg-um-bg font-sans">
            <Header />
            <main class="flex-1 overflow-hidden flex flex-col md:flex-row gap-3 p-3 md:p-4 min-h-0">
                <InputPanel
                    text_value={(*text).clone()}
                    on_text_change={on_text_change}
                    on_file={on_file}
                    active_tab={(*active_tab).clone()}
                    on_tab_change={on_tab_change}
                />
                <OutputPanel
                    result={(*result).clone()}
                    loading={false}
                />
            </main>
        </div>
    }
}

fn run_clean_text(text: &str) -> CleanResult {
    use cum_rs::unicode::{CleanOpts, clean_text};
    let opts = CleanOpts {
        aggressive_confusables: true,
        ..CleanOpts::safe()
    };
    match clean_text(text, &opts) {
        Ok((cleaned, raw)) => CleanResult {
            bytes: cleaned.into_bytes(),
            stats: CleanStats {
                removed_count: raw.removed_count,
                replaced_count: raw.replaced_count,
                metadata_chunks_removed: raw.metadata_chunks_removed,
                summary: raw.summary,
            },
            kind: MediaKind::Text,
            image_data_url: None,
        },
        Err(e) => CleanResult {
            bytes: format!("Error: {e}").into_bytes(),
            stats: CleanStats::default(),
            kind: MediaKind::Text,
            image_data_url: None,
        },
    }
}

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
