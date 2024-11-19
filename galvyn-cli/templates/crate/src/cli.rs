use clap::Parser;
use clap::Subcommand;

/// The cli
#[derive(Parser)]
pub struct Cli {
    /// The path to the config file
    #[clap(long, default_value_t = String::from("/etc/{{crate_name}}/config.toml"))]
    pub config_path: String,

    /// The available subcommands
    #[clap(subcommand)]
    pub command: Command,
}

/// All available commands
#[derive(Subcommand)]
pub enum Command {
    /// Start the server
    Start,
    /// Run the migrations on the database
    Migrate {
        /// The directory where the migration files are located in
        migrations_dir: String,
    },
    /// Create new migrations
    #[cfg(debug_assertions)]
    MakeMigrations {
        /// The directory where the migration files are located in
        migrations_dir: String,
    },
}
