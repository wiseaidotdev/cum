// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Controls Panel
//!
//! A sidebar panel that exposes the stochastic synonym-replacement settings
//! and the Layer A `normalize_punctuation` flag.
//!
//! ## Controls
//!
//! | Control | ID | Description |
//! |---------|----|-------------|
//! | Toggle  | `stochastic-toggle`       | Enable / disable synonym replacement. |
//! | Slider  | `prob-slider`             | Per-word substitution probability (1 - 100 %). |
//! | Select  | `language-select`         | Target language for the curated synonym table. |
//! | Toggle  | `normalize-punct-toggle`  | Enable / disable punctuation normalisation (Layer A). |
//!
//! Every change fires the parent callback immediately (no debounce) so the
//! output panel reflects the new setting on each interaction.

use crate::types::{AppLanguage, StochasticConfig};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement, MouseEvent};
use yew::prelude::*;

/// Props for [`ControlsPanel`].
#[derive(Properties, PartialEq)]
pub struct ControlsPanelProps {
    /// Current stochastic configuration.
    pub config: StochasticConfig,
    /// Fired immediately whenever any control value changes.
    pub on_change: Callback<StochasticConfig>,
}

#[function_component(ControlsPanel)]
pub fn controls_panel(props: &ControlsPanelProps) -> Html {
    let on_toggle = {
        let config = props.config.clone();
        let cb = props.on_change.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit(StochasticConfig {
                enabled: !config.enabled,
                ..config.clone()
            });
        })
    };

    let on_norm_punct_toggle = {
        let config = props.config.clone();
        let cb = props.on_change.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit(StochasticConfig {
                normalize_punctuation: !config.normalize_punctuation,
                ..config.clone()
            });
        })
    };

    let on_prob_input = {
        let config = props.config.clone();
        let cb = props.on_change.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(el) = e
                .target()
                .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
            {
                if let Ok(v) = el.value().parse::<u32>() {
                    cb.emit(StochasticConfig {
                        probability_pct: v.clamp(1, 100),
                        ..config.clone()
                    });
                }
            }
        })
    };

    let on_language_change = {
        let config = props.config.clone();
        let cb = props.on_change.clone();
        Callback::from(move |e: Event| {
            if let Some(el) = e
                .target()
                .and_then(|t| t.dyn_into::<HtmlSelectElement>().ok())
            {
                cb.emit(StochasticConfig {
                    language: AppLanguage::from_value(&el.value()),
                    ..config.clone()
                });
            }
        })
    };

    let stoch_track_class = if props.config.enabled {
        "um-toggle-track bg-um-accent"
    } else {
        "um-toggle-track bg-um-border"
    };
    let stoch_thumb_class = if props.config.enabled {
        "um-toggle-thumb translate-x-4"
    } else {
        "um-toggle-thumb translate-x-0"
    };

    let norm_track_class = if props.config.normalize_punctuation {
        "um-toggle-track bg-um-accent"
    } else {
        "um-toggle-track bg-um-border"
    };
    let norm_thumb_class = if props.config.normalize_punctuation {
        "um-toggle-thumb translate-x-4"
    } else {
        "um-toggle-thumb translate-x-0"
    };

    let all_languages = [
        AppLanguage::Auto,
        AppLanguage::English,
        AppLanguage::Spanish,
        AppLanguage::French,
        AppLanguage::German,
        AppLanguage::Arabic,
    ];

    html! {
        <aside class="um-controls-panel flex flex-col gap-4" aria-label="Cleaner controls">
            <div class="flex flex-col shrink-0">
                <div class="flex items-center gap-2 mb-2">
                    <i class="fa-solid fa-wand-magic-sparkles text-um-accent text-sm" aria-hidden="true"/>
                    <span class="text-sm font-semibold text-um-text">{"Layer A · Unicode"}</span>
                </div>
                <p class="text-xs text-um-muted mb-3 leading-relaxed">
                    {"Strips zero-width chars, private-use, tag blocks, variation selectors,\
                      homoglyphs, and (optionally) normalises punctuation to ASCII equivalents."}
                </p>
                <div class="flex items-center justify-between gap-2">
                    <label
                        for="normalize-punct-toggle"
                        class="text-sm text-um-text cursor-pointer select-none"
                    >
                        {"Normalise punctuation"}
                        <span class="block text-xs text-um-muted font-normal">
                            {"\u{201C}quotes\u{201D}, em\u{2013}dashes, ellipsis\u{2026}"}
                        </span>
                    </label>
                    <button
                        id="normalize-punct-toggle"
                        role="switch"
                        aria-checked={props.config.normalize_punctuation.to_string()}
                        aria-label="Toggle punctuation normalisation"
                        onclick={on_norm_punct_toggle}
                        class={norm_track_class}
                    >
                        <span class={norm_thumb_class} />
                    </button>
                </div>
            </div>
            <hr class="border-um-border shrink-0"/>
            <div class="flex-1 flex flex-col min-h-0 overflow-y-auto custom-scrollbar pr-1 pb-1">
                <div class="flex flex-col shrink-0">
                    <div class="flex items-center gap-2 mb-2">
                        <i class="fa-solid fa-shuffle text-um-accent text-sm" aria-hidden="true"/>
                        <span class="text-sm font-semibold text-um-text">{"Layer B · Synonyms"}</span>
                    </div>
                    <p class="text-xs text-um-muted mb-3 leading-relaxed">
                        {"Best-effort countermeasure for token-sampling watermarks (SynthID-Text, KGW). \
                          Replaces eligible words with semantically equivalent synonyms."}
                    </p>

                    <div class="flex flex-col gap-4">
                        <div class="flex items-center justify-between gap-2">
                            <label
                                for="stochastic-toggle"
                                class="text-sm text-um-text cursor-pointer select-none"
                            >
                                {"Enable synonym replacement"}
                            </label>
                            <button
                                id="stochastic-toggle"
                                role="switch"
                                aria-checked={props.config.enabled.to_string()}
                                aria-label="Toggle synonym replacement"
                                onclick={on_toggle}
                                class={stoch_track_class}
                            >
                                <span class={stoch_thumb_class} />
                            </button>
                        </div>

                        if props.config.enabled {
                            <div class="flex flex-col gap-4 animate-fade-in-up">
                                <div class="flex flex-col gap-1.5">
                                    <label for="language-select" class="text-sm text-um-text">
                                        {"Language"}
                                    </label>
                                    <select
                                        id="language-select"
                                        class="w-full rounded-lg border border-um-border bg-um-surface \
                                               text-um-text text-sm px-2 py-1.5 focus:outline-none \
                                               focus:ring-1 focus:ring-um-accent"
                                        onchange={on_language_change}
                                        aria-label="Synonym language"
                                    >
                                        { for all_languages.iter().map(|lang| html! {
                                            <option
                                                value={lang.value()}
                                                selected={props.config.language == *lang}
                                            >
                                                {lang.label()}
                                            </option>
                                        }) }
                                    </select>
                                </div>

                                <div class="flex flex-col gap-1.5">
                                    <div class="flex items-center justify-between">
                                        <label for="prob-slider" class="text-sm text-um-text">
                                            {"Probability"}
                                        </label>
                                        <span
                                            class="text-sm font-mono font-semibold text-um-accent"
                                            aria-live="polite"
                                        >
                                            {format!("{}%", props.config.probability_pct)}
                                        </span>
                                    </div>
                                    <input
                                        type="range"
                                        id="prob-slider"
                                        min="1"
                                        max="100"
                                        value={props.config.probability_pct.to_string()}
                                        oninput={on_prob_input}
                                        class="w-full h-1.5 rounded-full cursor-pointer accent-um-accent bg-um-border"
                                        aria-label="Synonym replacement probability"
                                        aria-valuemin="1"
                                        aria-valuemax="100"
                                        aria-valuenow={props.config.probability_pct.to_string()}
                                    />
                                    <p class="text-xs text-um-muted">
                                        {format!(
                                            "{}% chance each eligible word is replaced.",
                                            props.config.probability_pct
                                        )}
                                    </p>
                                </div>
                            </div>
                        }
                    </div>
                </div>
            </div>
        </aside>
    }
}
