//! The role management for users is defined in this module

use rorm::Model;

/// The role of a user
///
/// It should not be modified as it is created at start of the application
#[derive(Model, Clone)]
pub struct Role {
    /// The value should only be used in conversions to and from [`UserRole`]
    #[rorm(primary_key, max_length = 255)]
    pub identifier: String,
}

/// The roles of a user
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
// Database conversion
#[derive(strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[allow(missing_docs)]
pub enum UserRole {
    Administrator,
    Internal,
}
