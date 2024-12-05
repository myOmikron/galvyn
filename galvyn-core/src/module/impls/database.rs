use crate::{InitError, Module, PreInitError};
use rorm::{Database, DatabaseConfiguration, DatabaseDriver};
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseConfig {
    pub postgres_db: String,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_user: String,
    pub postgres_password: String,
}

impl Module for Database {
    type PreInit = DatabaseConfiguration;

    fn pre_init() -> impl Future<Output = Result<Self::PreInit, PreInitError>> + Send {
        async move {
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
    }

    type Dependencies = ();

    fn init(
        config: Self::PreInit,
        _dependencies: &mut Self::Dependencies,
    ) -> impl Future<Output = Result<Self, InitError>> + Send {
        async move { Ok(Database::connect(config).await?) }
    }
}
