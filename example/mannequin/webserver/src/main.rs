//! # mannequin

#![warn(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::env;
use std::fs;
use std::io;
use std::io::Write;

use clap::Parser;
use rorm::cli as rorm_cli;
use rorm::config::DatabaseConfig;
use rorm::Database;
use rorm::DatabaseConfiguration;
use tracing::instrument;
use webauthn_rs::WebauthnBuilder;

use crate::cli::Cli;
use crate::cli::Command;
use crate::config::Config;
use crate::global::ws::GlobalWs;
use crate::global::GlobalEntities;
use crate::global::GLOBAL;
use crate::http::handler_frontend::users::schema::UserLanguage;
use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::models::UserInvite;
use crate::utils::checked_string::CheckedString;
use crate::utils::links::new_user_invite_link;

mod cli;
pub mod config;
pub mod global;
pub mod http;
pub mod models;
pub mod utils;

#[instrument(skip_all)]
async fn start(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the database
    let mut conf = DatabaseConfiguration::new(config.database.clone().into());
    conf.disable_logging = Some(true);
    let db = Database::connect(conf).await?;

    let ws = GlobalWs::new();

    let webauthn = WebauthnBuilder::new(&config.webauthn.id, &config.webauthn.origin)?
        .rp_name(&config.webauthn.name)
        .build()?;
    let webauthn_attestation_ca_list = serde_json::from_reader(io::BufReader::new(
        fs::File::open(&config.webauthn.attestation_ca_list)?,
    ))?;

    // Initialize Globals
    GLOBAL.init(GlobalEntities {
        db,
        ws,
        webauthn,
        webauthn_attestation_ca_list,
        origin: config.server.origin.trim_end_matches('/').to_string(),
    });

    // Start the webserver
    http::server::run(config).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "INFO");
    }
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let config = Config::try_from_path(&cli.config_path)?;

    match cli.command {
        Command::Start => start(&config).await?,
        #[cfg(debug_assertions)]
        Command::MakeMigrations { migrations_dir } => {
            use std::io::Write;

            const MODELS: &str = ".models.json";

            let mut file = std::fs::File::create(MODELS)?;
            rorm::write_models(&mut file)?;
            file.flush()?;

            rorm_cli::make_migrations::run_make_migrations(
                rorm_cli::make_migrations::MakeMigrationsOptions {
                    models_file: MODELS.to_string(),
                    migration_dir: migrations_dir,
                    name: None,
                    non_interactive: false,
                    warnings_disabled: false,
                },
            )?;

            std::fs::remove_file(MODELS)?;
        }
        Command::Migrate { migrations_dir } => {
            rorm_cli::migrate::run_migrate_custom(
                DatabaseConfig {
                    driver: config.database.into(),
                    last_migration_table_name: None,
                },
                migrations_dir,
                false,
                None,
            )
            .await?
        }
        Command::CreateAdminUser => {
            create_admin_user(config).await?;
        }
    }

    Ok(())
}

/// Creates an invitation for an admin user
async fn create_admin_user(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the database
    let mut conf = DatabaseConfiguration::new(config.database.clone().into());
    conf.disable_logging = Some(true);
    let db = Database::connect(conf).await?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut mail = String::new();
    let mut display_name = String::new();

    print!("Enter a mail: ");
    stdout.flush()?;
    stdin.read_line(&mut mail)?;
    let mail = mail.trim();

    print!("Enter a display name: ");
    stdout.flush()?;
    stdin.read_line(&mut display_name)?;
    let display_name = display_name.trim().to_string();

    let invite = UserInvite::create(
        &db,
        CheckedString::new(mail.to_string()).map_err(|e| format!("Invalid mail: {e}"))?,
        CheckedString::new(display_name).map_err(|e| format!("Invalid display_name: {e}"))?,
        UserLanguage::EN,
        UserPermissions::Administrator,
    )
    .await?;

    println!(
        "Created invitation for {mail}, please go to {}",
        new_user_invite_link(config.server.origin.trim_end_matches('/'), invite.uuid)
    );

    db.close().await;
    Ok(())
}
