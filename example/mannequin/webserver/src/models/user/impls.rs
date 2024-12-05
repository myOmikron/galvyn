use rorm::db::Executor;
use rorm::delete;
use rorm::insert;
use rorm::prelude::ForeignModelByField;
use rorm::query;
use rorm::update;
use rorm::FieldAccess;
use rorm::Model;
use thiserror::Error;
use time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;
use webauthn_rs::prelude::AttestedPasskey;
use webauthn_rs::prelude::Passkey;

use crate::http::handler_frontend::users::schema::UserLanguage;
use crate::http::handler_frontend::users::schema::UserPermissions;
use crate::models::MaybeAttestedPasskey;
use crate::models::User;
use crate::models::UserInsert;
use crate::models::UserInvite;
use crate::models::UserInviteInsert;
use crate::models::UserRole;
use crate::utils::checked_string::CheckedString;

impl MaybeAttestedPasskey {
    /// Shorthand to access the `Passkey`
    ///
    /// (`AttestedPasskey` is just a specialization which can always be converted into the base type)
    pub fn passkey(self) -> Passkey {
        match self {
            Self::NotAttested(passkey) => passkey,
            Self::Attested(attested) => attested.into(),
        }
    }

    /// Shorthand to access the `AttestedPasskey`
    pub fn attested(self) -> Option<AttestedPasskey> {
        match self {
            Self::NotAttested(_) => None,
            Self::Attested(attested) => Some(attested),
        }
    }
}

impl User {
    /// Create a new user
    ///
    /// This function only creates the `User` model and the association required to store its permissions.
    /// The caller has to ensure a `LocalUser` or `OidcUser` is created after calling this function.
    ///
    /// The returned `Uuid` is the new `User`'s primary key.
    /// If it has been specified by the caller using the `uuid` argument,
    /// this will simply return the same value.
    pub async fn create(
        executor: impl Executor<'_>,
        mail: CheckedString<1, 255>,
        display_name: CheckedString<1, 255>,
        preferred_lang: UserLanguage,
        permissions: UserPermissions,
        uuid: Option<Uuid>,
    ) -> Result<Uuid, CreateUserError> {
        let mut guard = executor.ensure_transaction().await?;

        let uuid = uuid.unwrap_or_else(Uuid::new_v4);
        let mail_exists = query!(guard.get_transaction(), User)
            .condition(User::F.mail.equals(&mail))
            .optional()
            .await?
            .is_some();
        if mail_exists {
            return Err(CreateUserError::MailOccupied);
        }

        let role = match &permissions {
            UserPermissions::Administrator => UserRole::Administrator,
            UserPermissions::Internal { .. } => UserRole::Internal,
        };

        insert!(guard.get_transaction(), User)
            .return_nothing()
            .single(&UserInsert {
                uuid,
                display_name: display_name.into_inner(),
                preferred_lang: preferred_lang.to_string(),
                role: ForeignModelByField::Key(role.to_string()),
                mail: mail.into_inner(),
            })
            .await?;

        Self::set_permissions_internal::<false>(guard.get_transaction(), uuid, permissions).await?;

        guard.commit().await?;
        Ok(uuid)
    }

    /// Sets a user's permission overwriting old ones
    pub async fn set_permissions(
        executor: impl Executor<'_>,
        user_uuid: Uuid,
        permissions: UserPermissions,
    ) -> Result<(), rorm::Error> {
        Self::set_permissions_internal::<true>(executor, user_uuid, permissions).await
    }

    /// Actual implementation of [`User::set_permissions`]
    ///
    /// The compile time argument `NEW_USER` indicates whether this function is called from
    /// [`User::create`] in which case it can skip a few queries.
    async fn set_permissions_internal<const NEW_USER: bool>(
        executor: impl Executor<'_>,
        user_uuid: Uuid,
        permissions: UserPermissions,
    ) -> Result<(), rorm::Error> {
        let mut guard = executor.ensure_transaction().await?;

        if !NEW_USER {
            let role = match &permissions {
                UserPermissions::Administrator => UserRole::Administrator,
                UserPermissions::Internal { .. } => UserRole::Internal,
            };

            update!(guard.get_transaction(), User)
                .set(User::F.role, ForeignModelByField::Key(role.to_string()))
                .condition(User::F.uuid.equals(user_uuid))
                .await?;
        }

        guard.commit().await
    }

    /// Deletes an existing user
    ///
    /// Returns `false`, if the user didn't exist.
    pub async fn delete(executor: impl Executor<'_>, user_uuid: Uuid) -> Result<bool, rorm::Error> {
        let mut guard = executor.ensure_transaction().await?;

        let num_deleted = delete!(guard.get_transaction(), User)
            .condition(User::F.uuid.equals(user_uuid))
            .await?;

        guard.commit().await?;
        Ok(num_deleted > 0)
    }
}

/// The error that might occur when creating a user
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum CreateUserError {
    #[error("Database error: {0}")]
    Database(#[from] rorm::Error),
    #[error("There's already a user with the chosen mail")]
    MailOccupied,
}

impl UserInvite {
    /// Creates a new user invite checking if the mail is already used (either by user or open invite).
    pub async fn create(
        executor: impl Executor<'_>,
        mail: CheckedString<1, 255>,
        display_name: CheckedString<1, 255>,
        preferred_lang: UserLanguage,
        permissions: UserPermissions,
    ) -> Result<Self, CreateUserInviteError> {
        let mut guard = executor.ensure_transaction().await?;

        let user_with_mail_exists = query!(guard.get_transaction(), (User::F.uuid,))
            .condition(User::F.mail.equals(&mail))
            .optional()
            .await?
            .is_some();
        if user_with_mail_exists {
            return Err(CreateUserInviteError::AlreadyUser);
        }
        let invite_with_mail_exists = query!(guard.get_transaction(), (UserInvite::F.uuid,))
            .condition(UserInvite::F.email.equals(&mail))
            .optional()
            .await?
            .is_some();
        if invite_with_mail_exists {
            return Err(CreateUserInviteError::AlreadyInvited);
        }

        let invite = insert!(guard.get_transaction(), UserInvite)
            .single(&UserInviteInsert {
                uuid: Uuid::new_v4(),
                display_name: display_name.into_inner(),
                preferred_lang: preferred_lang.to_string(),
                email: mail.into_inner(),
                permissions: permissions.into(),
                expires_at: OffsetDateTime::now_utc() + Duration::days(1),
            })
            .await?;

        guard.commit().await?;
        Ok(invite)
    }
}
/// The error that might occur when creating a user invite
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum CreateUserInviteError {
    #[error("Database error: {0}")]
    Database(#[from] rorm::Error),
    #[error("There's already a user with the chosen mail")]
    AlreadyUser,
    #[error("There's already an open invite with the chosen mail")]
    AlreadyInvited,
}
