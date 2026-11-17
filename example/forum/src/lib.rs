use std::fs;
use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::Context;
use clap::Parser;
use clap::Subcommand;
use galvyn::core::modules::database::DatabaseSetup;
use galvyn::rorm;
use galvyn::Galvyn;
use rorm::Database;
use rorm::DatabaseConfiguration;

mod handler;
mod models;

/// The cli
#[derive(Parser)]
pub struct Cli {
    /// The path to the database config file
    #[clap(long)]
    pub db_config: String,

    /// The available subcommands
    #[clap(subcommand)]
    pub command: Command,
}

/// All available commands
#[derive(Subcommand)]
pub enum Command {
    /// Start the server
    Start,

    /// Create new migrations
    #[cfg(debug_assertions)]
    MakeMigrations {
        /// The directory where the migration files are located in
        migrations_dir: String,
    },
}

pub async fn run_main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Start => {
            let db_driver = serde_json::from_reader(
                fs::File::open(cli.db_config).context("Failed to open db config")?,
            )
            .context("Failed to read db config")?;

            // TODO: migrate

            Galvyn::builder(Default::default())
                .register_module::<Database>(DatabaseSetup::Custom(DatabaseConfiguration::new(
                    db_driver,
                )))
                .init_modules()
                .await?
                .add_listener(
                    SocketAddr::from_str("127.0.0.1:8080")?,
                    handler::get_router(),
                )
                .start()
                .await?;
        }
        #[cfg(debug_assertions)]
        Command::MakeMigrations { migrations_dir } => {
            use std::io::Write;

            const MODELS: &str = ".models.json";

            let mut file = fs::File::create(MODELS)?;
            rorm::write_models(&mut file)?;
            file.flush()?;

            rorm::cli::make_migrations::run_make_migrations(
                rorm::cli::make_migrations::MakeMigrationsOptions {
                    models_file: MODELS.to_string(),
                    migration_dir: migrations_dir,
                    name: None,
                    non_interactive: false,
                    warnings_disabled: false,
                },
            )?;

            fs::remove_file(MODELS)?;
        }
    }

    Ok(())
}
