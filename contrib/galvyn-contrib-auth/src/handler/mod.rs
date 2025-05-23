use galvyn_core::re_exports::axum::extract::Query;
use galvyn_core::re_exports::axum::Json;
use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::ApiError;
use galvyn_core::stuff::api_error::ApiResult;
use galvyn_core::Module;
use galvyn_macros::get;
use galvyn_macros::post;
use serde::Deserialize;
use serde::Serialize;
use webauthn_rs::prelude::AttestedPasskeyAuthentication;

use crate::handler::schema::GetLoginFlowsRequest;
use crate::handler::schema::GetLoginFlowsResponse;
use crate::handler::schema::LocalLoginFlow;
use crate::handler::schema::LoginLocalPasswordRequest;
use crate::handler::schema::LoginLocalWebauthnRequest;
use crate::handler::schema::OidcLoginFlow;
use crate::handler::schema::PublicKeyCredential;
use crate::handler::schema::RequestChallengeResponse;
use crate::models::LocalAccount;
use crate::models::OidcAccount;
use crate::models::WebAuthnKey;
use crate::module::AuthModule;
use crate::Account;
use crate::MaybeAttestedPasskey;

#[cfg(feature = "oidc")]
mod oidc;
#[cfg(feature = "oidc")]
pub use self::oidc::*;

mod local;
pub use self::local::*;
mod schema;

#[get("/login", core_crate = "::galvyn_core")]
pub async fn get_login_flow(
    Query(request): Query<GetLoginFlowsRequest>,
) -> ApiResult<Json<Option<GetLoginFlowsResponse>>> {
    let mut tx = AuthModule::global().db.start_transaction().await?;

    let Some(account_pk) = rorm::query(&mut tx, Account.pk)
        .condition(Account.id.equals(request.identifier.as_str()))
        .optional()
        .await?
    else {
        return Ok(Json(None));
    };

    let oidc = rorm::query(&mut tx, OidcAccount.account)
        .condition(OidcAccount.account.equals(&account_pk))
        .optional()
        .await?;

    let local = rorm::query(&mut tx, (LocalAccount.pk, LocalAccount.password))
        .condition(LocalAccount.account.equals(&account_pk))
        .optional()
        .await?;

    let response = match (oidc, local) {
        (Some(_), None) => GetLoginFlowsResponse::Oidc(OidcLoginFlow {}),
        (None, Some((local_pk, password))) => {
            let webauthn = rorm::query(&mut tx, WebAuthnKey.key)
                .condition(WebAuthnKey.local_account.equals(&local_pk))
                .all()
                .await?
                .into_iter()
                .any(|key| matches!(key.0, MaybeAttestedPasskey::Attested(_)));

            GetLoginFlowsResponse::Local(LocalLoginFlow {
                password: password.is_some(),
                webauthn,
            })
        }
        _ => return Err(ApiError::server_error("Invalid account")),
    };

    tx.commit().await?;
    Ok(Json(Some(response)))
}

#[post("/login/local/start-webauthn", core_crate = "::galvyn_core")]
pub async fn login_local_webauthn(
    session: Session,
    Json(request): Json<LoginLocalWebauthnRequest>,
) -> ApiResult<Json<RequestChallengeResponse>> {
    let mut tx = AuthModule::global().db.start_transaction().await?;

    let account_pk = rorm::query(&mut tx, Account.pk)
        .condition(Account.id.equals(&request.identifier))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Account not found"))?;

    let local_account_pk = rorm::query(&mut tx, LocalAccount.pk)
        .condition(LocalAccount.account.equals(&account_pk))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Not a local account"))?;

    let keys = rorm::query(&mut tx, WebAuthnKey.key)
        .condition(WebAuthnKey.local_account.equals(&local_account_pk))
        .all()
        .await?;
    let keys = keys
        .into_iter()
        .filter_map(|json| match json.0 {
            MaybeAttestedPasskey::NotAttested(_) => None,
            MaybeAttestedPasskey::Attested(key) => Some(key),
        })
        .collect::<Vec<_>>();

    let (challenge, state) = AuthModule::global()
        .webauthn
        .start_attested_passkey_authentication(&keys)
        .map_err(ApiError::map_server_error(
            "Failed to start webauthn challenge",
        ))?;

    tx.commit().await?;

    session
        .insert(
            "login_local_webauthn",
            LoginLocalWebauthnSessionData {
                identifier: request.identifier,
                state,
            },
        )
        .await?;

    Ok(Json(RequestChallengeResponse(challenge)))
}

#[derive(Serialize, Deserialize)]
struct LoginLocalWebauthnSessionData {
    identifier: String,
    state: AttestedPasskeyAuthentication,
}

#[post("/login/local/finish-webauthn", core_crate = "::galvyn_core")]
pub async fn finish_login_local_webauthn(
    session: Session,
    Json(request): Json<PublicKeyCredential>,
) -> ApiResult<()> {
    let LoginLocalWebauthnSessionData { identifier, state } = session
        .remove("login_local_webauthn")
        .await?
        .ok_or(ApiError::bad_request("No ongoing challenge"))?;

    let authentication_result = AuthModule::global()
        .webauthn
        .finish_attested_passkey_authentication(&request.0, &state)
        .map_err(ApiError::map_server_error(
            "Failed to finish webauthn challenge",
        ))?;

    let mut tx = AuthModule::global().db.start_transaction().await?;

    let account_pk = rorm::query(&mut tx, Account.pk)
        .condition(Account.id.equals(&identifier))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Account not found"))?;

    let local_account_pk = rorm::query(&mut tx, LocalAccount.pk)
        .condition(LocalAccount.account.equals(&account_pk))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Not a local account"))?;

    let keys = rorm::query(&mut tx, WebAuthnKey.key)
        .condition(WebAuthnKey.local_account.equals(&local_account_pk))
        .all()
        .await?;
    let _used_key = keys
        .into_iter()
        .find_map(|json| match json.0 {
            MaybeAttestedPasskey::NotAttested(_) => None,
            MaybeAttestedPasskey::Attested(key) => {
                (key.cred_id() == authentication_result.cred_id()).then_some(key)
            }
        })
        .ok_or(ApiError::bad_request("Used unknown key"))?;

    tx.commit().await?;

    session.insert("account", account_pk).await?;

    Ok(())
}

#[post("/login/local/password", core_crate = "::galvyn_core")]
pub async fn login_local_password(
    session: Session,
    Json(request): Json<LoginLocalPasswordRequest>,
) -> ApiResult<()> {
    let mut tx = AuthModule::global().db.start_transaction().await?;

    let account_pk = rorm::query(&mut tx, Account.pk)
        .condition(Account.id.equals(&request.identifier))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Account not found"))?;

    let local_account_password = rorm::query(&mut tx, LocalAccount.password)
        .condition(LocalAccount.account.equals(&account_pk))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Not a local account"))?;

    let local_account_password =
        local_account_password.ok_or(ApiError::bad_request("Account has no password"))?;
    // TODO: hashing
    if local_account_password != request.password {
        return Err(ApiError::bad_request("Passwords do not match"));
    }

    // TODO: 2nd factor

    tx.commit().await?;

    session.insert("account", account_pk).await?;

    Ok(())
}

#[post("/logout", core_crate = "::galvyn_core")]
pub async fn logout(session: Session) -> ApiResult<()> {
    session.remove::<serde::de::IgnoredAny>("account").await?;
    Ok(())
}
