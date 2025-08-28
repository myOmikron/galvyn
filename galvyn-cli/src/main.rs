use std::process::exit;

use clap::Parser;

use crate::cli::Cli;
use crate::cli::Command;
use crate::output::print_stacktrace;

pub mod cli;
pub mod commands;
pub mod output;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let res = match cli.command {
        Command::Init { name, path } => commands::run_init(name, path),
        Command::Module { name, path } => commands::run_module(name, path),
    };

    if let Err(err) = res {
        print_stacktrace(err);
        exit(1);
    }

    Ok(())
}
