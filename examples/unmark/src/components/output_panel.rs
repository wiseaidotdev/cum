// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Right-side output panel.

use crate::components::stats_badge::StatsBadge;
use crate::types::CleanResult;
use gloo_utils::window;
use wasm_bindgen::JsCast;
use yew::prelude::*;

/// Props for the right-side output panel.
#[derive(Properties, PartialEq)]
pub struct OutputPanelProps {
    /// The latest clean result, or `None` before any operation has run.
    pub result: Option<CleanResult>,
    /// Whether a cleaning operation is currently in flight (shows spinner).
    pub loading: bool,
}

#[function_component(OutputPanel)]
pub fn output_panel(props: &OutputPanelProps) -> Html {
    let copied = use_state(|| false);

    let on_copy = {
        let result = props.result.clone();
        let copied = copied.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(r) = &result {
                if let Ok(text) = std::str::from_utf8(&r.bytes) {
                    let _ = window().navigator().clipboard().write_text(text);
                }
                copied.set(true);
                let copied2 = copied.clone();
                gloo_timers::callback::Timeout::new(2000, move || copied2.set(false)).forget();
            }
        })
    };

    let on_download = {
        let result = props.result.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(r) = &result {
                let (mime, ext) = match &r.kind {
                    crate::types::MediaKind::Text => ("text/plain", "txt"),
                    crate::types::MediaKind::Image(m) => {
                        let ext = if m.contains("png") {
                            "png"
                        } else if m.contains("jpeg") || m.contains("jpg") {
                            "jpg"
                        } else if m.contains("webp") {
                            "webp"
                        } else {
                            "svg"
                        };
                        (m.as_str(), ext)
                    }
                    crate::types::MediaKind::Document(n) => {
                        (n.as_str(), n.rsplit('.').next().unwrap_or("bin"))
                    }
                };
                let arr = js_sys::Uint8Array::from(r.bytes.as_slice());
                let parts = js_sys::Array::new();
                parts.push(&arr);
                let opts = web_sys::BlobPropertyBag::new();
                opts.set_type(mime);
                if let Ok(blob) =
                    web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts)
                    && let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob)
                    && let Some(doc) = window().document()
                    && let Ok(a) = doc.create_element("a")
                {
                    let anchor = a.unchecked_into::<web_sys::HtmlElement>();
                    anchor.set_attribute("href", &url).ok();
                    anchor
                        .set_attribute("download", &format!("cleaned.{ext}"))
                        .ok();
                    anchor.click();
                    web_sys::Url::revoke_object_url(&url).ok();
                }
            }
        })
    };

    html! {
        <div class="um-panel flex-1 min-h-0">
            <div class="um-panel-header">
                <div class="flex items-center gap-2">
                    <i class="fa-solid fa-wand-magic-sparkles text-um-accent text-sm"/>
                    <span class="text-sm font-medium text-um-text">{"Cleaned Output"}</span>
                </div>
                if let Some(r) = &props.result {
                    <div class="flex items-center gap-1.5 md:gap-2">
                        if matches!(r.kind, crate::types::MediaKind::Text) {
                            <button class="um-btn-ghost text-xs px-2.5 md:px-3" onclick={on_copy} id="btn-copy">
                                if *copied {
                                    <i class="fa-solid fa-check text-emerald-400 text-xs"/>
                                    <span class="hidden sm:inline">{"Copied!"}</span>
                                } else {
                                    <i class="fa-regular fa-copy text-xs"/>
                                    <span class="hidden sm:inline">{"Copy"}</span>
                                }
                            </button>
                        }
                        <button class="um-btn-ghost text-xs px-2.5 md:px-3" onclick={on_download} id="btn-download">
                            <i class="fa-solid fa-download text-xs"/>
                            <span class="hidden sm:inline">{"Download"}</span>
                        </button>
                    </div>
                }
            </div>

            <div class="flex-1 overflow-auto p-3 md:p-5 min-h-0 flex flex-col gap-4">
                { render_output(props, *copied) }
            </div>
        </div>
    }
}

fn render_output(props: &OutputPanelProps, _copied: bool) -> Html {
    match &props.result {
        None => html! {
            <div class="flex-1 flex flex-col items-center justify-center gap-3 text-um-muted select-none py-8">
                <div class="w-14 h-14 md:w-16 md:h-16 rounded-2xl bg-um-elevated flex items-center justify-center">
                    <i class="fa-solid fa-broom text-2xl md:text-3xl text-um-muted"/>
                </div>
                <p class="text-sm font-medium text-um-muted">{"Output will appear here"}</p>
                <p class="text-xs text-um-subtle text-center max-w-[220px] md:max-w-[240px]">
                    {"Paste text or drop a file."}
                </p>
            </div>
        },

        Some(r) => html! {
            <>
                <StatsBadge stats={r.stats.clone()} kind={r.kind.clone()} />
                <div class="flex-1 min-h-0 animate-fade-in-up">
                    { render_content(r) }
                </div>
            </>
        },
    }
}

fn render_content(r: &CleanResult) -> Html {
    match &r.kind {
        crate::types::MediaKind::Image(_) => {
            if let Some(url) = &r.image_data_url {
                html! {
                    <img
                        src={url.clone()}
                        alt="Cleaned image"
                        class="max-w-full max-h-full object-contain rounded-xl border border-um-border"
                    />
                }
            } else {
                html! {
                    <p class="text-sm text-um-muted">
                        <i class="fa-solid fa-image mr-2 text-um-accent"/>
                        {"Image cleaned — click Download."}
                    </p>
                }
            }
        }
        _ => {
            let text = String::from_utf8_lossy(&r.bytes).to_string();
            html! {
                <pre class="text-xs md:text-sm text-um-text leading-relaxed whitespace-pre-wrap break-words font-mono bg-um-elevated rounded-xl p-3 md:p-4 min-h-[120px] md:min-h-[200px] overflow-auto">
                    { text }
                </pre>
            }
        }
    }
}
