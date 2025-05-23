use rorm::Database;
use rorm::DatabaseConfiguration;
use rorm::DatabaseDriver;
use serde::Deserialize;
use serde::Serialize;

use crate::InitError;
use crate::Module;
use crate::PreInitError;

/// Config struct the [`DatabaseSetup::Default`] will deserialize from environment variables
#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseConfig {
    pub postgres_db: String,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_user: String,
    pub postgres_password: String,
}

/// Enum declaring how the database should be configured
#[derive(Default, Debug)]
pub enum DatabaseSetup {
    #[default]
    Default,
    Custom(DatabaseConfiguration),
}

impl Module for Database {
    type Setup = DatabaseSetup;

    type PreInit = DatabaseConfiguration;

    async fn pre_init(setup: Self::Setup) -> Result<Self::PreInit, PreInitError> {
        match setup {
            DatabaseSetup::Default => {
                let DatabaseConfig {
                    postgres_db,
                    postgres_host,
                    postgres_port,
                    postgres_user,
                    postgres_password,
                } = envy::from_env()?;

                Ok(DatabaseConfiguration::new(DatabaseDriver::Postgres {
                    name: postgres_db,
                    host: postgres_host,
                    port: postgres_port,
                    user: postgres_user,
                    password: postgres_password,
                }))
            }
            DatabaseSetup::Custom(config) => Ok(config),
        }
    }

    type Dependencies = ();

    async fn init(
        config: Self::PreInit,
        _dependencies: &mut Self::Dependencies,
    ) -> Result<Self, InitError> {
        Ok(Database::connect(config).await?)
    }
}
