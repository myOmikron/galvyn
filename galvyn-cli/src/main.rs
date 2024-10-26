use std::process::exit;

use clap::Parser;
use owo_colors::colored::Color;
use owo_colors::OwoColorize;

use crate::cli::Cli;
use crate::cli::Command;
use crate::output::print_err;

pub mod cli;
pub mod commands;
mod output;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { name } => {
            commands::run_init(name);
        }
        Command::Module { name, path } => {
            if let Err(err) = commands::run_module(name, path) {
                print_err(&format!("{err}\n"));

                println!("{}", "Error chain:".color(Color::Red));
                for link in err.chain() {
                    println!("\t{link}");
                }

                exit(1);
            }
        }
    }

    Ok(())
}
