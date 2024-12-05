use rorm::db::Executor;
use rorm::prelude::ForeignModelByField;
use rorm::query;
use rorm::update;
use rorm::FieldAccess;
use rorm::Model;
use time::Duration;
use time::OffsetDateTime;
use tower_sessions::Session;
use tracing::trace;
use uuid::Uuid;

use crate::http::common::errors::ApiError;
use crate::http::common::errors::ApiResult;
use crate::http::session_keys::PartiallyAuthedSessionUser;
use crate::http::session_keys::PARTIALLY_AUTHED_SESSION_USER;
use crate::http::session_keys::SESSION_USER;
use crate::models;
use crate::models::LocalUser;

const MFA_TIMEOUT: Duration = Duration::minutes(10);

pub async fn get_partial_session_user(session: &Session) -> ApiResult<Uuid> {
    let Some(PartiallyAuthedSessionUser {
        timestamp,
        local_user,
    }) = session.get(PARTIALLY_AUTHED_SESSION_USER).await?
    else {
        trace!("{PARTIALLY_AUTHED_SESSION_USER} is missing in session");
        return Err(ApiError::Unauthenticated);
    };

    if OffsetDateTime::now_utc() - timestamp > MFA_TIMEOUT {
        trace!("{PARTIALLY_AUTHED_SESSION_USER} expired");
        return Err(ApiError::Unauthenticated);
    }

    Ok(local_user)
}

pub async fn set_partial_session_user(session: &Session, local_user_uuid: Uuid) -> ApiResult<()> {
    session
        .insert(
            PARTIALLY_AUTHED_SESSION_USER,
            PartiallyAuthedSessionUser {
                timestamp: OffsetDateTime::now_utc(),
                local_user: local_user_uuid,
            },
        )
        .await?;
    Ok(())
}

pub async fn set_session_user(
    executor: impl Executor<'_>,
    session: &Session,
    local_user_uuid: Uuid,
) -> ApiResult<()> {
    let mut guard = executor.ensure_transaction().await?;

    let Some((ForeignModelByField::Key(user_uuid),)) =
        query!(guard.get_transaction(), (LocalUser::F.user,))
            .condition(LocalUser::F.uuid.equals(local_user_uuid))
            .optional()
            .await?
    else {
        return Err(ApiError::Unauthenticated);
    };

    session
        .remove::<serde::de::IgnoredAny>(PARTIALLY_AUTHED_SESSION_USER)
        .await?;
    session.insert(SESSION_USER, user_uuid).await?;
    session.save().await?;

    let Some(id) = session.id() else {
        return Err(ApiError::new_internal_server_error("No ID in session"));
    };
    update!(guard.get_transaction(), models::Session)
        .condition(models::Session::F.id.equals(id.to_string()))
        .set(
            models::Session::F.user,
            Some(ForeignModelByField::Key(user_uuid)),
        )
        .exec()
        .await?;

    guard.commit().await?;
    Ok(())
}
