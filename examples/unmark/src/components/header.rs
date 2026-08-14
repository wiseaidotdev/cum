// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Header component.

use yew::prelude::*;

#[function_component(Header)]
pub fn header() -> Html {
    html! {
        <header class="flex items-center justify-between px-4 md:px-6 py-3 md:py-4 border-b border-um-border bg-um-surface/80 backdrop-blur-sm sticky top-0 z-50 shrink-0">
            <div class="flex items-center gap-2 md:gap-3">
                <div class="w-8 h-8 md:w-9 md:h-9 rounded-xl bg-um-accent/15 flex items-center justify-center shrink-0">
                    <i class="fa-solid fa-broom text-um-accent text-sm md:text-base"/>
                </div>
                <div>
                    <h1 class="text-sm md:text-base font-bold text-um-text leading-tight tracking-tight">
                        {"CUM: Claude Unmarking Machine"}
                    </h1>
                    <p class="text-[10px] md:text-xs text-um-muted leading-none hidden sm:block">
                        {"AI watermark remover · 100% client-side"}
                    </p>
                </div>
            </div>

            <div class="flex items-center gap-2 md:gap-3">
                <span class="hidden md:inline-flex items-center gap-1.5 text-xs text-um-muted px-3 py-1.5 rounded-full bg-um-elevated border border-um-border">
                    <i class="fa-solid fa-lock text-emerald-400 text-[10px]"/>
                    {"No data leaves your device"}
                </span>
                <a
                    href="https://github.com/wiseaidotdev/cum"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="um-btn-ghost px-2.5 md:px-3"
                    aria-label="GitHub repository"
                >
                    <i class="fa-brands fa-github text-base"/>
                    <span class="hidden sm:inline text-sm">{"GitHub"}</span>
                </a>
            </div>
        </header>
    }
}
