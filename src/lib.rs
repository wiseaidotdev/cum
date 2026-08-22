// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/wiseaidotdev/cum/main/assets/favicon.png",
    html_logo_url = "https://raw.githubusercontent.com/wiseaidotdev/cum/main/assets/logo.webp"
)]
#![doc = include_str!("../README.md")]
#![doc = include_str!("../RUST.md")]
#![doc = include_str!("../WASM.md")]

//! # `cum-rs`: Claude Unmarking Machine
//!
//! A multilanguage watermark removal crate for AI-generated content. The core
//! is pure Rust; Python, Node.js, and WASM bindings are built via Cargo
//! feature flags.

pub mod cleaner;
pub mod error;
pub mod types;

pub mod container;
pub mod image;
pub mod text;

pub use container as container_meta;
pub use image::meta as image_meta;
pub use text::stochastic;
pub use text::unicode;

#[cfg(all(feature = "pixel-scrub", not(target_arch = "wasm32")))]
pub use image::pixel_scrub;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub mod wasm;

#[cfg(all(feature = "python", not(feature = "rust-binary")))]
pub mod python;

#[cfg(all(feature = "node", not(feature = "rust-binary")))]
pub mod node;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(all(feature = "python", not(feature = "rust-binary")))]
use pyo3::prelude::*;

#[cfg(all(feature = "python", not(feature = "rust-binary")))]
#[pymodule(name = "cum_rs")]
fn cum_rs(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    crate::python::register_python_module(py, m)?;
    Ok(())
}
