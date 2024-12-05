//! The handler for the users

use axum::extract::Path;
use futures::TryStreamExt;
use rorm::and;
use rorm::insert;
use rorm::prelude::ForeignModelByField;
use rorm::query;
use rorm::update;
use rorm::FieldAccess;
use rorm::Model;
use swaggapi::delete;
use swaggapi::get;
use swaggapi::post;
use swaggapi::utils::SchemalessJson;
use tower_sessions::Session;
use tracing::debug;
use tracing::instrument;
use uuid::Uuid;
use webauthn_rs::prelude::CreationChallengeResponse;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

use crate::global::GLOBAL;
use crate::http::common::errors::ApiError;
use crate::http::common::errors::ApiResult;
use crate::http::common::schemas::FormResult;
use crate::http::common::schemas::List;
use crate::http::common::schemas::SingleUuid;
use crate::http::extractors::api_json::ApiJson;
use crate::http::extractors::session_user::SessionUser;
use crate::http::handler_frontend::users::schema::ChangePwFormErrors;
use crate::http::handler_frontend::users::schema::ChangePwRequest;
use crate::http::handler_frontend::users::schema::CreateTotpErrors;
use crate::http::handler_frontend::users::schema::CreateTotpRequest;
use crate::http::handler_frontend::users::schema::CreateTotpSecretError;
use crate::http::handler_frontend::users::schema::CreateWebAuthnRequest;
use crate::http::handler_frontend::users::schema::FullUser;
use crate::http::handler_frontend::users::schema::SimpleTotpKey;
use crate::http::handler_frontend::users::schema::SimpleWebAuthnKey;
use crate::http::handler_frontend::users::utils::new_full_user;
use crate::http::session_keys::WebAuthnRegistration;
use crate::http::session_keys::WebAuthnRegistrationState;
use crate::http::session_keys::SESSION_WEBAUTHN_REGISTRATION;
use crate::models::LocalUser;
use crate::models::MaybeAttestedPasskey;
use crate::models::TotpKey;
use crate::models::TotpKeyInsert;
use crate::models::WebAuthnKey;
use crate::models::WebAuthnKeyInsert;
use crate::utils::checked_string::CheckedString;
use crate::utils::hashing;
use crate::utils::hashing::hash_pw;
use crate::utils::hashing::VerifyPwError;
use crate::utils::schemars::SchemaDateTime;
use crate::utils::totp::totp_from_base32;
use crate::utils::totp::TotpFromError;
use crate::utils::webauthn::WebAuthnRegisterResult;

/// Retrieve the currently logged-in user
#[get("/me")]
#[instrument(skip_all)]
pub async fn get_me(SessionUser { user, .. }: SessionUser) -> ApiResult<ApiJson<FullUser>> {
    new_full_user(user).map(ApiJson)
}

/// Change the password of the currently logged-in user
///
/// This may only be called by local users
#[post("/me/change-pw")]
#[instrument(skip_all, ret, err)]
pub async fn change_password(
    SessionUser { user, .. }: SessionUser,
    ApiJson(ChangePwRequest { current_pw, new_pw }): ApiJson<ChangePwRequest>,
) -> ApiResult<ApiJson<FormResult<(), ChangePwFormErrors>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let Some(local_user) = query!(&mut tx, LocalUser)
        .condition(LocalUser::F.user.equals(user.uuid))
        .optional()
        .await?
    else {
        debug!("Change password was requested from a not-local user");
        return Err(ApiError::BadRequest);
    };

    let Some(old_hashed_password) = local_user.password.as_deref() else {
        debug!("Change password was requested from a password less user");
        return Err(ApiError::BadRequest);
    };

    if let Err(err) = hashing::verify_pw(&current_pw, &old_hashed_password) {
        return match err {
            VerifyPwError::Hash(error) => Err(error.into()),
            VerifyPwError::Mismatch => Ok(ApiJson(FormResult::err(ChangePwFormErrors {
                current_pw: true,
                ..Default::default()
            }))),
        };
    }

    let hashed = hash_pw(&new_pw)?;

    update!(&mut tx, LocalUser)
        .condition(LocalUser::F.user.equals(user.uuid))
        .set(LocalUser::F.password, Some(hashed))
        .exec()
        .await?;

    tx.commit().await?;

    Ok(ApiJson(FormResult::ok(())))
}

/// Adds a TOTP key to the logged-in user.
///
/// This may only be called by local users.
#[post("/me/totp")]
#[instrument(skip_all, ret, err)]
pub async fn create_totp_key(
    SessionUser { user, .. }: SessionUser,
    ApiJson(request): ApiJson<CreateTotpRequest>,
) -> ApiResult<ApiJson<FormResult<SingleUuid, CreateTotpErrors>>> {
    let totp = match totp_from_base32(&request.secret) {
        Ok(totp) => totp,
        Err(TotpFromError::InvalidBase32) => {
            return Ok(ApiJson(FormResult::err(CreateTotpErrors {
                secret: Some(CreateTotpSecretError::InvalidBase32),
                ..Default::default()
            })))
        }
        Err(TotpFromError::InvalidSecret(_)) => {
            return Ok(ApiJson(FormResult::err(CreateTotpErrors {
                secret: Some(CreateTotpSecretError::InvalidRfc6238),
                ..Default::default()
            })))
        }
        Err(error @ TotpFromError::Unreachable(_)) => {
            return Err(ApiError::new_internal_server_error(error));
        }
    };

    if !totp.check_current(&request.token)? {
        return Ok(ApiJson(FormResult::err(CreateTotpErrors {
            token: true,
            ..Default::default()
        })));
    }

    let mut tx = GLOBAL.db.start_transaction().await?;

    let Some((local_user_uuid,)) = query!(&mut tx, (LocalUser::F.uuid,))
        .condition(LocalUser::F.user.equals(user.uuid))
        .optional()
        .await?
    else {
        debug!("TOTP add request was requested from a not-local user");
        return Err(ApiError::BadRequest);
    };

    let uuid = insert!(&mut tx, TotpKey)
        .return_primary_key()
        .single(&TotpKeyInsert {
            uuid: Uuid::new_v4(),
            local_user: ForeignModelByField::Key(local_user_uuid),
            secret: totp.secret,
            label: request.label.into_inner(),
        })
        .await?;

    tx.commit().await?;
    Ok(ApiJson(FormResult::ok(SingleUuid { uuid })))
}

/// Retrieves TOTP keys for the logged-in user.
///
/// This may only be called by local users.
#[get("/me/totp")]
#[instrument(skip_all, ret, err)]
pub async fn list_totp_keys(
    SessionUser { user, .. }: SessionUser,
) -> ApiResult<ApiJson<List<SimpleTotpKey>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let (local_user_uuid,) = query!(&mut tx, (LocalUser::F.uuid,))
        .condition(LocalUser::F.user.equals(user.uuid))
        .optional()
        .await?
        .ok_or(ApiError::BadRequest)?;

    let mut list = Vec::new();
    let mut stream = query!(
        &mut tx,
        (TotpKey::F.uuid, TotpKey::F.label, TotpKey::F.created_at)
    )
    .condition(TotpKey::F.local_user.equals(local_user_uuid))
    .stream();
    while let Some((uuid, label, created_at)) = stream.try_next().await? {
        list.push(SimpleTotpKey {
            uuid,
            label: CheckedString::new(label).unwrap(),
            created_at: SchemaDateTime(created_at),
        });
    }
    drop(stream);

    tx.commit().await?;
    Ok(ApiJson(List { list }))
}

/// Removes a totp key
///
/// This may only be called by local users.
#[delete("/me/totp/:uuid")]
#[instrument(skip_all, ret, err)]
pub async fn delete_totp_key(
    SessionUser { user, .. }: SessionUser,
    Path(SingleUuid { uuid: key_uuid }): Path<SingleUuid>,
) -> ApiResult<()> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let Some((local_user_uuid,)) = query!(&mut tx, (LocalUser::F.uuid,))
        .condition(LocalUser::F.user.equals(user.uuid))
        .optional()
        .await?
    else {
        debug!("TOTP remove request was requested from a not-local user");
        return Err(ApiError::BadRequest);
    };

    rorm::delete!(&mut tx, TotpKey)
        .condition(and![
            TotpKey::F.uuid.equals(key_uuid),
            TotpKey::F.local_user.equals(local_user_uuid),
        ])
        .await?;

    tx.commit().await?;

    Ok(())
}

/// Adds a WebAuthn key to the logged-in user.
///
/// This may only be called by local users.
#[post("/me/webauthn")]
#[instrument(skip_all, ret, err)]
pub async fn create_webauthn_key(
    session: Session,
    SessionUser { user, .. }: SessionUser,
    ApiJson(request): ApiJson<CreateWebAuthnRequest>,
) -> ApiResult<SchemalessJson<CreationChallengeResponse>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let Some((local_user_uuid,)) = query!(&mut tx, (LocalUser::F.uuid,))
        .condition(LocalUser::F.user.equals(user.uuid))
        .optional()
        .await?
    else {
        debug!("TOTP add request was requested from a not-local user");
        return Err(ApiError::BadRequest);
    };

    let known_keys = query!(&GLOBAL.db, (WebAuthnKey::F.key,))
        .condition(WebAuthnKey::F.local_user.equals(local_user_uuid))
        .stream()
        .map_ok(|(key,)| key.0.passkey().cred_id().clone())
        .try_collect()
        .await?;

    let (challenge, state) = if request.can_login {
        let (challenge, state) = GLOBAL.webauthn.start_attested_passkey_registration(
            user.uuid,
            &user.mail,
            &user.display_name,
            Some(known_keys),
            GLOBAL.webauthn_attestation_ca_list.clone(),
            None,
        )?;
        (challenge, WebAuthnRegistrationState::Attested(state))
    } else {
        let (challenge, state) = GLOBAL.webauthn.start_passkey_registration(
            user.uuid,
            &user.mail,
            &user.display_name,
            Some(known_keys),
        )?;
        (challenge, WebAuthnRegistrationState::NotAttested(state))
    };

    session
        .insert(
            SESSION_WEBAUTHN_REGISTRATION,
            WebAuthnRegistration {
                label: request.label,
                local_user: local_user_uuid,
                state,
            },
        )
        .await?;

    Ok(SchemalessJson(challenge))
}

/// Complete the webauthn challenge for registering a new key
#[post("/me/complete-webauthn")]
#[instrument(skip_all, ret, err)]
pub async fn complete_users_webauthn(
    session: Session,
    SchemalessJson(request): SchemalessJson<RegisterPublicKeyCredential>,
) -> ApiResult<ApiJson<WebAuthnRegisterResult>> {
    let WebAuthnRegistration {
        label,
        local_user,
        state,
    } = session
        .remove(SESSION_WEBAUTHN_REGISTRATION)
        .await?
        .ok_or(ApiError::BadRequest)?;

    let webauthn_result = match state {
        WebAuthnRegistrationState::NotAttested(state) => GLOBAL
            .webauthn
            .finish_passkey_registration(&request, &state)
            .map(MaybeAttestedPasskey::NotAttested),
        WebAuthnRegistrationState::Attested(state) => GLOBAL
            .webauthn
            .finish_attested_passkey_registration(&request, &state)
            .map(MaybeAttestedPasskey::Attested),
    };
    let key = match webauthn_result {
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

    insert!(&GLOBAL.db, WebAuthnKey)
        .single(&WebAuthnKeyInsert {
            uuid: Uuid::new_v4(),
            local_user: ForeignModelByField::Key(local_user),
            label: label.into_inner(),
            key: key.into(),
        })
        .await?;

    Ok(ApiJson(WebAuthnRegisterResult::Ok))
}

/// Retrieves WebAuthn keys for the logged-in user.
///
/// This may only be called by local users.
#[get("/me/webauthn")]
pub async fn list_webauthn_keys(
    SessionUser { user, .. }: SessionUser,
) -> ApiResult<ApiJson<List<SimpleWebAuthnKey>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let (local_user_uuid,) = query!(&mut tx, (LocalUser::F.uuid,))
        .condition(LocalUser::F.user.equals(user.uuid))
        .optional()
        .await?
        .ok_or(ApiError::BadRequest)?;

    let mut list = Vec::new();
    let mut stream = query!(
        &mut tx,
        (
            WebAuthnKey::F.uuid,
            WebAuthnKey::F.label,
            WebAuthnKey::F.created_at,
            WebAuthnKey::F.key,
        )
    )
    .condition(WebAuthnKey::F.local_user.equals(local_user_uuid))
    .stream();
    while let Some((uuid, label, created_at, key)) = stream.try_next().await? {
        list.push(SimpleWebAuthnKey {
            uuid,
            label: CheckedString::new(label).unwrap(),
            created_at: SchemaDateTime(created_at),
            can_login: key.0.attested().is_some(),
        });
    }
    drop(stream);

    tx.commit().await?;
    Ok(ApiJson(List { list }))
}

/// Removes a totp key
///
/// This may only be called by local users.
#[delete("/me/webauthn/:uuid")]
pub async fn delete_webauthn_key(
    SessionUser { user, .. }: SessionUser,
    Path(SingleUuid { uuid: key_uuid }): Path<SingleUuid>,
) -> ApiResult<()> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let (local_user_uuid,) = query!(&mut tx, (LocalUser::F.uuid,))
        .condition(LocalUser::F.user.equals(user.uuid))
        .optional()
        .await?
        .ok_or(ApiError::BadRequest)?;

    rorm::delete!(&mut tx, WebAuthnKey)
        .condition(and![
            WebAuthnKey::F.uuid.equals(key_uuid),
            WebAuthnKey::F.local_user.equals(local_user_uuid),
        ])
        .await?;

    tx.commit().await?;

    Ok(())
}
