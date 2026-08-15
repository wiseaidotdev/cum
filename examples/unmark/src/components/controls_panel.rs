// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Controls Panel
//!
//! A sidebar panel that exposes the stochastic synonym-replacement settings.
//!
//! ## Controls
//!
//! | Control | ID | Description |
//! |---------|----|-------------|
//! | Toggle  | `stochastic-toggle`    | Enable / disable synonym replacement. |
//! | Slider  | `prob-slider`          | Per-word substitution probability (1 – 100 %). |
//!
//! Every change fires the parent callback immediately (no debounce) so the
//! output panel reflects the new setting on each interaction.

use crate::types::StochasticConfig;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, MouseEvent};
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
                probability_pct: config.probability_pct,
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
                        enabled: config.enabled,
                        probability_pct: v.clamp(1, 100),
                    });
                }
            }
        })
    };

    let track_class = if props.config.enabled {
        "um-toggle-track bg-um-accent"
    } else {
        "um-toggle-track bg-um-border"
    };

    let thumb_class = if props.config.enabled {
        "um-toggle-thumb translate-x-4"
    } else {
        "um-toggle-thumb translate-x-0"
    };

    html! {
        <aside class="um-controls-panel" aria-label="Stochastic controls">
            <div class="flex items-center gap-2 mb-3">
                <i class="fa-solid fa-shuffle text-um-accent text-sm" aria-hidden="true"/>
                <span class="text-sm font-semibold text-um-text">{"Stochastic Layer B"}</span>
            </div>
            <p class="text-xs text-um-muted mb-3 leading-relaxed">
                {"Best-effort countermeasure for token-sampling watermarks (SynthID, KGW). \
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
                        class={track_class}
                    >
                        <span class={thumb_class} />
                    </button>
                </div>

                if props.config.enabled {
                    <div class="flex flex-col gap-1.5 animate-fade-in-up">
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
                                "{}% chance each eligible word is replaced by a synonym.",
                                props.config.probability_pct
                            )}
                        </p>
                    </div>
                }
            </div>
        </aside>
    }
}
