// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(feature = "cli")]
use anyhow::Result;
#[cfg(feature = "cli")]
use clap::Parser;
#[cfg(feature = "cli")]
use cum_rs::cli::{Cli, run};

#[cfg(feature = "cli")]
fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("Rebuild with --features rust-binary to enable the CLI.");
    std::process::exit(1);
}
