// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Text Watermark Removal
//!
//! This module groups all **text-domain** watermark detection and removal:
//!
//! - [`unicode`]: Layer A deterministic Unicode scrubbing.
//! - [`stochastic`]: Layer B stochastic synonym substitution.

pub mod stochastic;
pub mod unicode;
