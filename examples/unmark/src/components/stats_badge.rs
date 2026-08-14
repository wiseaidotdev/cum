// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::types::{CleanStats, MediaKind};
use yew::prelude::*;

/// Props for the stats badge overlay shown in the output panel.
#[derive(Properties, PartialEq)]
pub struct StatsBadgeProps {
    /// Statistics from the last cleaning operation.
    pub stats: CleanStats,
    /// The media kind that was cleaned.
    pub kind: MediaKind,
}

#[function_component(StatsBadge)]
pub fn stats_badge(props: &StatsBadgeProps) -> Html {
    let total = props.stats.total_marks();

    html! {
        <div class="flex flex-wrap items-center gap-2 animate-fade-in-up">
            if total == 0 {
                <span class="stat-chip stat-chip-ok">
                    <i class="fa-solid fa-circle-check text-emerald-400 text-xs"/>
                    {"No watermarks found"}
                </span>
            } else {
                <span class="stat-chip stat-chip-warn">
                    <i class="fa-solid fa-triangle-exclamation text-red-400 text-xs"/>
                    {format!("{total} watermark{} removed", if total == 1 { "" } else { "s" })}
                </span>
            }

            if props.stats.removed_count > 0 {
                <span class="stat-chip">
                    <i class="fa-solid fa-eraser text-um-muted text-xs"/>
                    {format!("{} stripped", props.stats.removed_count)}
                </span>
            }

            if props.stats.replaced_count > 0 {
                <span class="stat-chip">
                    <i class="fa-solid fa-arrows-rotate text-um-muted text-xs"/>
                    {format!("{} replaced", props.stats.replaced_count)}
                </span>
            }

            if props.stats.metadata_chunks_removed > 0 {
                <span class="stat-chip">
                    <i class="fa-solid fa-tags text-um-muted text-xs"/>
                    {format!("{} metadata chunk{} stripped",
                        props.stats.metadata_chunks_removed,
                        if props.stats.metadata_chunks_removed == 1 { "" } else { "s" }
                    )}
                </span>
            }

            <span class={props.kind.badge_class()}>
                <i class={format!("{} text-[10px]", props.kind.icon())}/>
                {props.kind.label()}
            </span>

            { for props.stats.summary.iter().map(|line| html! {
                <span class="w-full text-xs text-um-subtle font-mono leading-relaxed">
                    { line.clone() }
                </span>
            })}
        </div>
    }
}
