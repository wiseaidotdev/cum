// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # WASM Bindings
//!
//! `wasm-bindgen` exports for use in browser environments (vanilla JS,
//! Node.js with the WASM target).
//!
//! ## Why no `#[wasm_bindgen(start)]`?
//!
//! `lib.rs` deliberately does **not** register a `#[wasm_bindgen(start)]`
//! entry point.  Library crates must not auto-run code at WASM module load
//! time because any host application (e.g. a Yew app) that **also** uses
//! `wasm-bindgen` will have its own startup routine, and two `start` symbols
//! in the same binary cause a link error.
//!
//! Instead, call [`init_panic_hook`] once from your own application startup:
//!
//! ```js
//! import init, { init_panic_hook } from "./cum_rs.js";
//! await init();
//! init_panic_hook();          // forward Rust panics to the browser console
//! ```
//!
//! ## Available Functions
//!
//! | Export | Description |
//! |--------|-------------|
//! | [`init_panic_hook`] | Sets up `console_error_panic_hook` (call once at startup). |
//! | [`clean_text_wasm`] | Layer-A clean of a JS string; returns a JSON object. |
//! | [`inspect_text_wasm`] | Layer-A inspection; returns a JSON object. |
//! | [`clean_bytes_wasm`] | Format-auto-detect clean of a `Uint8Array`. |
//! | [`inspect_bytes_wasm`] | Format-auto-detect inspection of a `Uint8Array`. |
//! | [`version`] | Returns the crate version string. |
//!
//! All functions serialise their return values as JSON via `serde_json` /
//! `serde-wasm-bindgen` so they are usable from vanilla JavaScript without any
//! additional Rust bindings.

use crate::cleaner::{clean, inspect};
use crate::unicode::{CleanOpts, InspectOpts, clean_text, inspect_text};
use wasm_bindgen::prelude::*;

/// Installs `console_error_panic_hook` so that Rust panics are forwarded to
/// the browser developer console as readable messages.
///
/// Call this **once** at startup from JavaScript:
/// ```js
/// import init, { init_panic_hook } from "./cum_rs.js";
/// await init();
/// init_panic_hook();
/// ```
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Removes all Layer-A Unicode watermark carriers from a JavaScript string.
///
/// # Returns
/// A `JsValue` that is a JSON object:
/// ```json
/// { "cleaned": "...", "removed_count": 3, "replaced_count": 1, "summary": [...] }
/// ```
/// On error, throws a `JsValue` with the error message string.
#[wasm_bindgen]
pub fn clean_text_wasm(text: &str) -> Result<JsValue, JsValue> {
    let opts = CleanOpts {
        aggressive_confusables: true,
        ..CleanOpts::safe()
    };
    let (cleaned, stats) =
        clean_text(text, &opts).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let obj = serde_json::json!({
        "cleaned": cleaned,
        "removed_count": stats.removed_count,
        "replaced_count": stats.replaced_count,
        "summary": stats.summary,
    });
    serde_wasm_bindgen::to_value(&obj).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Inspects a JavaScript string for Layer-A Unicode watermark carriers.
///
/// # Returns
/// A `JsValue` that is a JSON [`crate::types::TextInspectReport`].
/// On error, throws a `JsValue` with the error message string.
#[wasm_bindgen]
pub fn inspect_text_wasm(text: &str) -> Result<JsValue, JsValue> {
    let opts = InspectOpts {
        aggressive_confusables: true,
        ..InspectOpts::default()
    };
    let report = inspect_text(text, &opts).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&report).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Removes all detectable watermarks from a `Uint8Array` (image or document).
///
/// The format is auto-detected from magic bytes.
///
/// # Returns
/// A `Uint8Array` containing the cleaned bytes.
/// On error, throws a `JsValue` with the error message string.
#[wasm_bindgen]
pub fn clean_bytes_wasm(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    clean(data, None)
        .map(|out| out.bytes)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Inspects a `Uint8Array` (image or document) for watermark findings.
///
/// The format is auto-detected from magic bytes.
///
/// # Returns
/// A `JsValue` that is a JSON serialisation of [`crate::types::InspectOutput`].
/// On error, throws a `JsValue` with the error message string.
#[wasm_bindgen]
pub fn inspect_bytes_wasm(data: &[u8]) -> Result<JsValue, JsValue> {
    let output = inspect(data, None).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let obj = serde_json::json!({
        "format":        format!("{:?}", output.format),
        "text_report":   output.text_report,
        "image_report":  output.image_report,
        "meta_findings": output.meta_findings,
    });
    serde_wasm_bindgen::to_value(&obj).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Returns the crate version string (e.g. `"0.1.0"`).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
