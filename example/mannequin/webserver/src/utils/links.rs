//! Collection of `format!` invocations to generate links
//!
//! This module serves as single source of truth for dynamically generated links.

use uuid::Uuid;

/// Constructs a new link for a user invite.
///
/// The link resolves to a view in the frontend where a user provides a login method
/// in order to accept the invite and create a user account.
pub fn new_user_invite_link(origin: &str, user_invite_uuid: Uuid) -> String {
    format!("{origin}/invite/{user_invite_uuid}")
}
