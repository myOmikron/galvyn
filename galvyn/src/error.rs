use std::io;

use thiserror::Error;

/// Error type for galvyn
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum GalvynError {
    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("{0}")]
    Init(#[from] galvyn_core::module::registry::builder::InitError),
}
