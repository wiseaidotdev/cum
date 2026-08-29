// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Build Script
//!
//! Emits link flags required by napi-rs when the `node` feature is enabled.

fn main() {
    #[cfg(feature = "node")]
    napi_build::setup();
}
