// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Left-side input panel.

use input_rs::yew::Input;
use wasm_bindgen::JsCast;
use web_sys::{HtmlTextAreaElement, MouseEvent};
use yew::prelude::*;

/// Props for the left-side input panel.
#[derive(Properties, PartialEq)]
pub struct InputPanelProps {
    /// Current text value in the controlled textarea.
    pub text_value: String,
    /// Emitted with the new text on every keystroke.
    pub on_text_change: Callback<String>,
    /// Emitted with `(filename, bytes)` pairs when files are picked or dropped.
    pub on_file: Callback<Vec<(String, Vec<u8>)>>,
    /// Active tab identifier: `"text"` or `"file"`.
    pub active_tab: String,
    /// Emitted when the user switches input tab.
    pub on_tab_change: Callback<String>,
    /// Emitted when the user clicks "Clear".
    pub on_clear: Callback<MouseEvent>,
}

#[function_component(InputPanel)]
pub fn input_panel(props: &InputPanelProps) -> Html {
    let drag_over = use_state(|| false);
    let input_ref = use_node_ref();
    let input_handle = use_state(String::default);
    let input_valid = use_state(|| true);

    {
        let r = input_ref.clone();
        let val = props.text_value.clone();
        use_effect_with(val.clone(), move |v| {
            if let Some(el) = r.cast::<HtmlTextAreaElement>() {
                if el.value() != *v {
                    el.set_value(v);
                }
            }
            || ()
        });
    }

    let on_input = {
        let cb = props.on_text_change.clone();
        let r = input_ref.clone();
        Callback::from(move |_: InputEvent| {
            if let Some(el) = r.cast::<HtmlTextAreaElement>() {
                cb.emit(el.value());
            }
        })
    };

    let on_dragover = {
        let drag = drag_over.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            drag.set(true);
        })
    };

    let on_dragleave = {
        let drag = drag_over.clone();
        Callback::from(move |_: DragEvent| drag.set(false))
    };

    let on_drop = {
        let drag = drag_over.clone();
        let on_file = props.on_file.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            drag.set(false);
            if let Some(dt) = e.data_transfer() {
                if let Some(files) = dt.files() {
                    read_file_list(files, on_file.clone());
                }
            }
        })
    };

    let on_file_pick = {
        let on_file = props.on_file.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            {
                if let Some(files) = input.files() {
                    read_file_list(files, on_file.clone());
                }
            }
        })
    };

    let tab_text_click = {
        let cb = props.on_tab_change.clone();
        Callback::from(move |_: MouseEvent| cb.emit("text".into()))
    };
    let tab_file_click = {
        let cb = props.on_tab_change.clone();
        Callback::from(move |_: MouseEvent| cb.emit("file".into()))
    };

    let drop_class = if *drag_over {
        "um-drop-zone um-drop-zone-active"
    } else {
        "um-drop-zone"
    };
    let text_tab_class = if props.active_tab == "text" {
        "um-tab um-tab-active"
    } else {
        "um-tab um-tab-inactive"
    };
    let file_tab_class = if props.active_tab == "file" {
        "um-tab um-tab-active"
    } else {
        "um-tab um-tab-inactive"
    };

    html! {
        <div class="um-panel flex-1 min-h-0">
            <div class="um-panel-header">
                <div class="flex items-center gap-1 bg-um-bg rounded-full p-1">
                    <button class={text_tab_class} onclick={tab_text_click} id="tab-text">
                        <i class="fa-solid fa-font mr-1 text-xs" />
                        { "Text" }
                    </button>
                    <button class={file_tab_class} onclick={tab_file_click} id="tab-file">
                        <i class="fa-solid fa-upload mr-1 text-xs" />
                        { "File" }
                    </button>
                </div>
                <div class="flex items-center gap-2">
                    <span class="text-xs text-um-subtle hidden sm:inline-flex items-center gap-1">
                        <i class="fa-solid fa-bolt text-um-accent text-[10px]" />
                        { "Live cleaning" }
                    </span>
                    if props.active_tab == "text" && !props.text_value.is_empty() {
                        <button
                            class="text-xs text-um-muted hover:text-um-text transition-colors"
                            onclick={props.on_clear.clone()}
                            id="clear-btn"
                            aria-label="Clear input"
                            type="button"
                        >
                            <i class="fa-solid fa-xmark mr-1" />
                            { "Clear" }
                        </button>
                    }
                </div>
            </div>
            <div class="flex-1 p-3 md:p-5 min-h-0">
                if props.active_tab == "text" {
                    <div oninput={on_input} class="h-full">
                        <Input
                            r#type="textarea"
                            label=""
                            handle={input_handle}
                            name="input-text"
                            r#ref={input_ref}
                            placeholder="Paste AI-generated text here...\n\nThe cleaner will remove:\n\u{2022} Zero-width spaces & invisible controls\n\u{2022} Bidirectional format characters\n\u{2022} Tag characters (U+E0001-U+E007F)\n\u{2022} Variation selectors & private-use chars\n\u{2022} Space homoglyphs, dash homoglyphs & confusable letters\n\u{2022} Curly quotes, em-dashes, ellipsis (with normalize_punctuation)"
                            input_class="um-textarea h-full"
                            field_class="h-full"
                            error_class=""
                            valid_handle={input_valid}
                            validate_function={Callback::from(|_: String| true)}
                            id="input-text"
                        />
                    </div>
                } else {
                    <div
                        class={drop_class}
                        ondragover={on_dragover}
                        ondragleave={on_dragleave}
                        ondrop={on_drop}
                    >
                        <i class="fa-solid fa-cloud-arrow-up text-3xl text-um-muted mb-4 block" />
                        <p class="text-sm text-um-text font-medium mb-1">
                            { "Drop a file or click to browse" }
                        </p>
                        <p class="text-xs text-um-muted mb-4">
                            { "PNG · JPEG · WebP · SVG · PDF · DOCX · ODT · HTML · Markdown" }
                        </p>
                        <label
                            class="um-btn-primary cursor-pointer w-full sm:w-auto justify-center"
                        >
                            <i class="fa-solid fa-folder-open text-sm" />
                            { "Browse files" }
                            <input
                                type="file"
                                class="hidden"
                                id="file-input"
                                accept=".png,.jpg,.jpeg,.webp,.svg,.pdf,.docx,.odt,.html,.htm,.md,.markdown"
                                onchange={on_file_pick}
                            />
                        </label>
                    </div>
                }
            </div>
        </div>
    }
}

fn read_file_list(files: web_sys::FileList, cb: Callback<Vec<(String, Vec<u8>)>>) {
    for i in 0..files.length() {
        if let Some(file) = files.item(i) {
            let name = file.name();
            let cb = cb.clone();
            let reader = web_sys::FileReader::new().unwrap();
            let reader_clone = reader.clone();
            let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Ok(val) = reader_clone.result() {
                    if let Some(ab) = val.dyn_ref::<js_sys::ArrayBuffer>() {
                        cb.emit(vec![(name.clone(), js_sys::Uint8Array::new(ab).to_vec())]);
                    }
                }
            }) as Box<dyn FnMut(_)>);
            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            onload.forget();
            reader.read_as_array_buffer(&file).ok();
        }
    }
}
