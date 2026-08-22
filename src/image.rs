// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Image Watermark Removal
//!
//! Groups all **image-domain** watermark detection and removal logic:
//!
//! - [`meta`]: Metadata scrubbing (EXIF, C2PA, XMP) for PNG, JPEG, WebP, SVG.
//! - [`pixel_scrub`]: Pixel-domain re-encoding to neutralise SynthID-Image,
//!   StegaStamp, Tree-Ring, and StableSignature embeddings.

pub mod meta;

#[cfg(all(feature = "pixel-scrub", not(target_arch = "wasm32")))]
pub mod pixel_scrub;
