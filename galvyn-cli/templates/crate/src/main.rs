//! # {{crate_name}}

#![warn(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use clap::Parser;

use crate::cli::Cli;

mod cli;
mod http;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
}
