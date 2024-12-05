//! Admin handlers for user invites

use axum::extract::Path;
use rorm::insert;
use rorm::prelude::ForeignModelByField;
use rorm::query;
use rorm::FieldAccess;
use rorm::Model;
use swaggapi::get;
use swaggapi::post;
use swaggapi::utils::SchemalessJson;
use time::OffsetDateTime;
use tower_sessions::Session;
use tracing::debug;
use uuid::Uuid;
use webauthn_rs::prelude::CreationChallengeResponse;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

use crate::global::GLOBAL;
use crate::http::common::errors::ApiError;
use crate::http::common::errors::ApiResult;
use crate::http::common::schemas::SingleUuid;
use crate::http::extractors::api_json::ApiJson;
use crate::http::handler_frontend::user_invites::schema::AcceptWithPwRequest;
use crate::http::handler_frontend::user_invites::schema::AcceptWithWARequest;
use crate::http::handler_frontend::user_invites::schema::GetUserInviteResponse;
use crate::http::handler_frontend::user_invites::utils::new_simple_user_invite;
use crate::http::handler_frontend::users::utils::set_logged_in;
use crate::http::session_keys::WebAuthnAccept;
use crate::http::session_keys::SESSION_WEBAUTHN_ACCEPT;
use crate::models::LocalUser;
use crate::models::LocalUserInsert;
use crate::models::MaybeAttestedPasskey;
use crate::models::User;
use crate::models::UserInvite;
use crate::models::WebAuthnKey;
use crate::models::WebAuthnKeyInsert;
use crate::utils::checked_string::CheckedString;
use crate::utils::hashing::hash_pw;
use crate::utils::webauthn::WebAuthnRegisterResult;

/// Gets an invitation's details to display to the user before accepting
#[get("/:uuid")]
pub async fn get_user_invite(
    Path(SingleUuid { uuid }): Path<SingleUuid>,
) -> ApiResult<ApiJson<GetUserInviteResponse>> {
    Ok(ApiJson(
        if let Some(invite) = query!(&GLOBAL.db, UserInvite)
            .condition(UserInvite::F.uuid.equals(uuid))
            .optional()
            .await?
        {
            if invite.expires_at < OffsetDateTime::now_utc() {
                GetUserInviteResponse::Expired
            } else {
                GetUserInviteResponse::Valid {
                    invite: new_simple_user_invite(invite)?,
                }
            }
        } else {
            GetUserInviteResponse::NotFound
        },
    ))
}

/// Accepts an invitation by providing a password for authentication
#[post("/accept/:uuid/with-password")]
pub async fn accept_with_password(
    session: Session,
    Path(SingleUuid { uuid }): Path<SingleUuid>,
    ApiJson(request): ApiJson<AcceptWithPwRequest>,
) -> ApiResult<()> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let invite = query!(&mut tx, UserInvite)
        .condition(UserInvite::F.uuid.equals(uuid))
        .optional()
        .await?
        .ok_or(ApiError::BadRequest)?;
    if invite.expires_at < OffsetDateTime::now_utc() {
        return Err(ApiError::BadRequest);
    }
    rorm::delete!(&mut tx, UserInvite).single(&invite).await?;

    let user_uuid = User::create(
        &mut tx,
        CheckedString::new(invite.email)?,
        CheckedString::new(invite.display_name)?,
        invite.preferred_lang.parse()?,
        invite.permissions.0,
        None,
    )
    .await
    // CreateUserError::MailOccupied shouldn't happen since the invite should check its uniqueness upon creation
    .map_err(ApiError::new_internal_server_error)?;

    insert!(&mut tx, LocalUser)
        .return_nothing()
        .single(&LocalUserInsert {
            uuid: Uuid::new_v4(),
            user: ForeignModelByField::Key(user_uuid),
            password: Some(hash_pw(&request.password)?),
        })
        .await?;

    set_logged_in(&mut tx, &session, user_uuid).await?;

    tx.commit().await?;
    Ok(())
}

/// Accepts an invitation by providing a webauthn key for authentication
#[post("/accept/:uuid/with-webauthn")]
pub async fn accept_with_webauthn(
    session: Session,
    Path(SingleUuid { uuid }): Path<SingleUuid>,
    ApiJson(request): ApiJson<AcceptWithWARequest>,
) -> ApiResult<SchemalessJson<CreationChallengeResponse>> {
    let invite = query!(&GLOBAL.db, UserInvite)
        .condition(UserInvite::F.uuid.equals(uuid))
        .optional()
        .await?
        .ok_or(ApiError::BadRequest)?;
    let user_uuid = Uuid::new_v4();
    let (challenge, state) = GLOBAL.webauthn.start_attested_passkey_registration(
        user_uuid,
        &invite.email,
        &invite.display_name,
        None,
        GLOBAL.webauthn_attestation_ca_list.clone(),
        None,
    )?;
    session
        .insert(
            SESSION_WEBAUTHN_ACCEPT,
            WebAuthnAccept {
                label: request.label,
                user_uuid,
                invite_uuid: uuid,
                state,
            },
        )
        .await?;

    Ok(SchemalessJson(challenge))
}

/// Complete the webauthn challenge for accepting the invite by registering a key
#[post("/complete-webauthn")]
pub async fn complete_invites_webauthn(
    session: Session,
    SchemalessJson(request): SchemalessJson<RegisterPublicKeyCredential>,
) -> ApiResult<ApiJson<WebAuthnRegisterResult>> {
    let WebAuthnAccept {
        label,
        user_uuid,
        invite_uuid,
        state,
    } = session
        .remove(SESSION_WEBAUTHN_ACCEPT)
        .await?
        .ok_or(ApiError::BadRequest)?;
    let passkey = match GLOBAL
        .webauthn
        .finish_attested_passkey_registration(&request, &state)
    {
        Ok(passkey) => passkey,
        Err(error) => {
            return if let Some(result) = WebAuthnRegisterResult::parse(&error) {
                Ok(ApiJson(result))
            } else {
                debug!(error.display = %error, error.debug = ?error, "WebAuthn Challenge failed");
                Err(ApiError::BadRequest)
            }
        }
    };

    let mut tx = GLOBAL.db.start_transaction().await?;

    let invite = query!(&mut tx, UserInvite)
        .condition(UserInvite::F.uuid.equals(invite_uuid))
        .optional()
        .await?
        .ok_or(ApiError::BadRequest)?;
    if invite.expires_at < OffsetDateTime::now_utc() {
        return Err(ApiError::BadRequest);
    }
    rorm::delete!(&mut tx, UserInvite).single(&invite).await?;

    User::create(
        &mut tx,
        CheckedString::new(invite.email)?,
        CheckedString::new(invite.display_name)?,
        invite.preferred_lang.parse()?,
        invite.permissions.0,
        Some(user_uuid),
    )
    .await
    // CreateUserError::MailOccupied shouldn't happen since the invite should check its uniqueness upon creation
    .map_err(ApiError::new_internal_server_error)?;

    let local_user_uuid = insert!(&mut tx, LocalUser)
        .return_primary_key()
        .single(&LocalUserInsert {
            uuid: Uuid::new_v4(),
            user: ForeignModelByField::Key(user_uuid),
            password: None,
        })
        .await?;

    insert!(&mut tx, WebAuthnKey)
        .single(&WebAuthnKeyInsert {
            uuid: Uuid::new_v4(),
            local_user: ForeignModelByField::Key(local_user_uuid),
            label: label.into_inner(),
            key: MaybeAttestedPasskey::Attested(passkey).into(),
        })
        .await?;

    set_logged_in(&mut tx, &session, user_uuid).await?;

    tx.commit().await?;
    Ok(ApiJson(WebAuthnRegisterResult::Ok))
}
