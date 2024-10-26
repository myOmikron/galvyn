use clap::Parser;
use clap::Subcommand;

/// CLI parser
#[derive(Parser)]
#[clap(author, version)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

/// All available commands
#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new configured project
    Init {
        /// Name of the project
        name: String,

        /// Path in which the new module should be created
        #[clap(short, long, default_value_t = String::from("./"))]
        path: String,
    },

    /// Create a new module
    Module {
        /// Name of the module
        name: String,

        /// Path in which the new module should be created
        #[clap(short, long, default_value_t = String::from("./src/http/"))]
        path: String,
    },
}
