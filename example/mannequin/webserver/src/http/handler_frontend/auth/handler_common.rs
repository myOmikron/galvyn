//! The handler for local authentication

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use futures::TryStreamExt;
use rorm::query;
use rorm::FieldAccess;
use rorm::Model;
use swaggapi::post;
use swaggapi::utils::SchemalessJson;
use tower_sessions::Session;
use tracing::debug;
use tracing::instrument;
use webauthn_rs::prelude::PublicKeyCredential;
use webauthn_rs::prelude::RequestChallengeResponse;

use crate::global::GLOBAL;
use crate::http::common::errors::ApiError;
use crate::http::common::errors::ApiResult;
use crate::http::common::schemas::FormResult;
use crate::http::common::schemas::Optional;
use crate::http::extractors::api_json::ApiJson;
use crate::http::handler_frontend::auth::schema::LoginFlowsRequest;
use crate::http::handler_frontend::auth::schema::LoginPasswordErrors;
use crate::http::handler_frontend::auth::schema::LoginPasswordRequest;
use crate::http::handler_frontend::auth::schema::LoginPasswordResponse;
use crate::http::handler_frontend::auth::schema::LoginWebauthnErrors;
use crate::http::handler_frontend::auth::schema::LoginWebauthnRequest;
use crate::http::handler_frontend::auth::schema::SupportedLoginFlows;
use crate::http::handler_frontend::auth::schema::VerifyTotpRequest;
use crate::http::handler_frontend::auth::schema::WebAuthnAuthenticateResult;
use crate::http::handler_frontend::auth::schema::MFA;
use crate::http::handler_frontend::auth::utils::get_partial_session_user;
use crate::http::handler_frontend::auth::utils::set_partial_session_user;
use crate::http::handler_frontend::auth::utils::set_session_user;
use crate::http::session_keys::WebAuthnAuthentication;
use crate::http::session_keys::WebAuthnAuthenticationState;
use crate::http::session_keys::SESSION_WEBAUTHN_AUTHENTICATION;
use crate::models::LocalUser;
use crate::models::OidcUser;
use crate::models::TotpKey;
use crate::models::User;
use crate::models::WebAuthnKey;
use crate::utils::hashing;
use crate::utils::hashing::VerifyPwError;
use crate::utils::schemars::WebAuthnSchema;
use crate::utils::totp;

/// Get the login flows available to a user
#[post("/flows")]
pub async fn get_login_flows(
    ApiJson(LoginFlowsRequest { mail }): ApiJson<LoginFlowsRequest>,
) -> ApiResult<ApiJson<Optional<SupportedLoginFlows>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let Some((user_uuid,)) = query!(&mut tx, (User::F.uuid,))
        .condition(User::F.mail.equals(&mail))
        .optional()
        .await?
    else {
        return Ok(ApiJson(Optional::none()));
    };

    let is_oidc = query!(&mut tx, (OidcUser::F.uuid))
        .condition(OidcUser::F.user.equals(user_uuid))
        .optional()
        .await?
        .is_some();
    if is_oidc {
        tx.commit().await?;
        return Ok(ApiJson(Optional::some(SupportedLoginFlows {
            mail,
            oidc: true,
            password: false,
            key: false,
        })));
    }

    let (local_user_uuid, password) = query!(&mut tx, (LocalUser::F.uuid, LocalUser::F.password,))
        .condition(LocalUser::F.user.equals(user_uuid))
        .optional()
        .await?
        .ok_or(ApiError::new_internal_server_error(
            "Invalid db state: user is neither oidc nor local",
        ))?;

    let mut key = false;
    let mut stream = query!(&mut tx, (WebAuthnKey::F.key,))
        .condition(WebAuthnKey::F.local_user.equals(local_user_uuid))
        .stream();
    while let Some((passkey,)) = stream.try_next().await? {
        if passkey.0.attested().is_some() {
            key = true;
            break;
        }
    }
    drop(stream);

    tx.commit().await?;
    Ok(ApiJson(Optional::some(SupportedLoginFlows {
        mail,
        oidc: false,
        password: password.is_some(),
        key,
    })))
}

/// Local login using webauthn
///
/// Doesn't require another factor
#[post("/login-webauthn")]
pub async fn login_webauthn(
    session: Session,
    ApiJson(request): ApiJson<LoginWebauthnRequest>,
) -> ApiResult<ApiJson<FormResult<WebAuthnSchema<RequestChallengeResponse>, LoginWebauthnErrors>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let Some(local_user) = query!(&mut tx, LocalUser)
        .condition(LocalUser::F.user.mail.equals(&request.mail))
        .optional()
        .await?
    else {
        return Ok(ApiJson(FormResult::err(LoginWebauthnErrors { mail: true })));
    };

    let keys = query!(&mut tx, (WebAuthnKey::F.key,))
        .condition(WebAuthnKey::F.local_user.equals(local_user.uuid))
        .stream()
        .try_filter_map(|(json,)| async move { Ok(json.0.attested()) })
        .try_collect::<Vec<_>>()
        .await?;

    let (challenge, state) = GLOBAL
        .webauthn
        .start_attested_passkey_authentication(&keys)?;

    session
        .insert(
            SESSION_WEBAUTHN_AUTHENTICATION,
            WebAuthnAuthentication {
                local_user: local_user.uuid,
                state: WebAuthnAuthenticationState::Attested(state),
            },
        )
        .await?;

    Ok(ApiJson(FormResult::ok(WebAuthnSchema(challenge))))
}

/// Local login using a password
///
/// Might require another factor
#[post("/login-password")]
pub async fn login_password(
    session: Session,
    ApiJson(request): ApiJson<LoginPasswordRequest>,
) -> ApiResult<ApiJson<FormResult<LoginPasswordResponse, LoginPasswordErrors>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let Some(local_user) = query!(&mut tx, LocalUser)
        .condition(LocalUser::F.user.mail.equals(&request.mail))
        .optional()
        .await?
    else {
        return Ok(ApiJson(FormResult::err(LoginPasswordErrors {
            mail: true,
            ..Default::default()
        })));
    };

    let Some(hashed_password) = local_user.password.as_deref() else {
        return Err(ApiError::BadRequest);
    };

    match hashing::verify_pw(&request.password, hashed_password) {
        Ok(()) => {}
        Err(VerifyPwError::Hash(error)) => return Err(error.into()),
        Err(VerifyPwError::Mismatch) => {
            return Ok(ApiJson(FormResult::err(LoginPasswordErrors {
                password: true,
                ..Default::default()
            })));
        }
    }

    let has_totp = query!(&mut tx, (TotpKey::F.uuid,))
        .condition(TotpKey::F.local_user.equals(local_user.uuid))
        .optional()
        .await?
        .is_some();
    let has_webauthn = query!(&mut tx, (WebAuthnKey::F.uuid,))
        .condition(WebAuthnKey::F.local_user.equals(local_user.uuid))
        .optional()
        .await?
        .is_some();

    if has_totp || has_webauthn {
        set_partial_session_user(&session, local_user.uuid).await?;

        tx.commit().await?;
        Ok(ApiJson(FormResult::ok(LoginPasswordResponse::NeedMFA {
            mfa: MFA {
                has_totp,
                has_webauthn,
            },
        })))
    } else {
        set_session_user(&mut tx, &session, local_user.uuid).await?;

        tx.commit().await?;
        Ok(ApiJson(FormResult::ok(LoginPasswordResponse::Finished)))
    }
}

/// Verify a password login using an WebAuthn key
#[post("/verify-webauthn")]
pub async fn verify_webauthn(
    session: Session,
) -> ApiResult<ApiJson<WebAuthnSchema<RequestChallengeResponse>>> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let local_user_uuid = get_partial_session_user(&session).await?;

    let keys = query!(&mut tx, (WebAuthnKey::F.key,))
        .condition(WebAuthnKey::F.local_user.equals(local_user_uuid))
        .stream()
        .map_ok(|(json,)| json.0.passkey())
        .try_collect::<Vec<_>>()
        .await?;

    let (challenge, state) = GLOBAL.webauthn.start_passkey_authentication(&keys)?;

    session
        .insert(
            SESSION_WEBAUTHN_AUTHENTICATION,
            WebAuthnAuthentication {
                local_user: local_user_uuid,
                state: WebAuthnAuthenticationState::NotAttested(state),
            },
        )
        .await?;

    Ok(ApiJson(WebAuthnSchema(challenge)))
}

/// Verify a password login using an TOTP key
#[post("/verify-totp")]
#[instrument(skip(session))]
pub async fn verify_totp(
    session: Session,
    ApiJson(request): ApiJson<VerifyTotpRequest>,
) -> ApiResult<()> {
    let mut tx = GLOBAL.db.start_transaction().await?;

    let local_user_uuid = get_partial_session_user(&session).await?;

    let keys = query!(&mut tx, TotpKey)
        .condition(TotpKey::F.local_user.equals(local_user_uuid))
        .all()
        .await?;

    let mut is_valid = false;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    for key in keys {
        let totp = totp::totp_from_binary(key.secret)?;
        if totp.check(&request.token, now) {
            is_valid = true;
            break;
        }
    }

    if !is_valid {
        return Err(ApiError::Unauthenticated); // TODO form error
    }

    set_session_user(&mut tx, &session, local_user_uuid).await?;

    tx.commit().await?;

    Ok(())
}

/// Complete the webauthn challenge for authentication
#[post("/complete-webauthn")]
pub async fn complete_auth_webauthn(
    session: Session,
    SchemalessJson(request): SchemalessJson<PublicKeyCredential>,
) -> ApiResult<ApiJson<WebAuthnAuthenticateResult>> {
    let WebAuthnAuthentication { local_user, state } = session
        .remove(SESSION_WEBAUTHN_AUTHENTICATION)
        .await?
        .ok_or(ApiError::BadRequest)?;

    let webauthn_result = match state {
        WebAuthnAuthenticationState::NotAttested(state) => GLOBAL
            .webauthn
            .finish_passkey_authentication(&request, &state),
        WebAuthnAuthenticationState::Attested(state) => GLOBAL
            .webauthn
            .finish_attested_passkey_authentication(&request, &state),
    };
    if let Err(error) = webauthn_result {
        debug!(error.display = %error, error.debug = ?error, "WebAuthn Challenge failed");
        return Ok(ApiJson(WebAuthnAuthenticateResult::Err));
    }

    set_session_user(&GLOBAL.db, &session, local_user).await?;

    Ok(ApiJson(WebAuthnAuthenticateResult::Ok))
}

/// Drop the current session and logg-out
#[post("/logout")]
#[instrument(skip_all)]
pub async fn logout(session: Session) -> ApiResult<()> {
    session.flush().await?;
    Ok(())
}
