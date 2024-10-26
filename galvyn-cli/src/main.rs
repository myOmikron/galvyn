use std::process::exit;

use clap::Parser;
use owo_colors::OwoColorize;

use crate::cli::Cli;
use crate::cli::Command;
use crate::output::print_stacktrace;

pub mod cli;
pub mod commands;
mod output;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { name, path } => {
            if let Err(err) = commands::run_init(name, path) {
                print_stacktrace(err);
                exit(1);
            }
        }
        Command::Module { name, path } => {
            if let Err(err) = commands::run_module(name, path) {
                print_stacktrace(err);
                exit(1);
            }
        }
    }

    Ok(())
}
