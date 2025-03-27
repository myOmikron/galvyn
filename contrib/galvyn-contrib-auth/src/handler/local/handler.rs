use crate::handler::local::schema::{
    FinishLoginLocalWebauthnRequest, LoginLocalPasswordRequest, LoginLocalWebauthnRequest,
    LoginLocalWebauthnResponse,
};
use crate::models::{LocalAccount, MaybeAttestedPasskey, WebAuthnKey};
use crate::{Account, AuthModule};
use galvyn_core::re_exports::axum::Json;
use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::{ApiError, ApiResult};
use galvyn_core::Module;
use galvyn_macros::{delete, post, put};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::AttestedPasskeyAuthentication;

#[post("/local/login/start-webauthn", core_crate = "::galvyn_core")]
pub async fn login_local_webauthn(
    session: Session,
    Json(request): Json<LoginLocalWebauthnRequest>,
) -> ApiResult<Json<LoginLocalWebauthnResponse>> {
    let mut tx = AuthModule::global().db.start_transaction().await?;

    let account_uuid = rorm::query(&mut tx, Account.uuid)
        .condition(Account.id.equals(&request.identifier))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Account not found"))?;

    let local_account_uuid = rorm::query(&mut tx, LocalAccount.uuid)
        .condition(LocalAccount.account.equals(&account_uuid))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Not a local account"))?;

    let keys = rorm::query(&mut tx, WebAuthnKey.key)
        .condition(WebAuthnKey.local_account.equals(&local_account_uuid))
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

    Ok(Json(LoginLocalWebauthnResponse(challenge)))
}

#[derive(Serialize, Deserialize)]
struct LoginLocalWebauthnSessionData {
    identifier: String,
    state: AttestedPasskeyAuthentication,
}

#[post("/local/login/finish-webauthn", core_crate = "::galvyn_core")]
pub async fn finish_login_local_webauthn(
    session: Session,
    Json(request): Json<FinishLoginLocalWebauthnRequest>,
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

    let account_uuid = rorm::query(&mut tx, Account.uuid)
        .condition(Account.id.equals(&identifier))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Account not found"))?;

    let local_account_uuid = rorm::query(&mut tx, LocalAccount.uuid)
        .condition(LocalAccount.account.equals(&account_uuid))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Not a local account"))?;

    let keys = rorm::query(&mut tx, WebAuthnKey.key)
        .condition(WebAuthnKey.local_account.equals(&local_account_uuid))
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

    session.insert("account", account_uuid).await?;

    Ok(())
}

#[post("/local/login/password", core_crate = "::galvyn_core")]
pub async fn login_local_password(
    session: Session,
    Json(request): Json<LoginLocalPasswordRequest>,
) -> ApiResult<()> {
    let mut tx = AuthModule::global().db.start_transaction().await?;

    let account_uuid = rorm::query(&mut tx, Account.uuid)
        .condition(Account.id.equals(&request.identifier))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("Account not found"))?;

    let local_account_password = rorm::query(&mut tx, LocalAccount.password)
        .condition(LocalAccount.account.equals(&account_uuid))
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

    session.insert("account", account_uuid).await?;

    Ok(())
}

type SetLocalPasswordRequest = String;

#[put("/local/password", core_crate = "::galvyn_core")]
pub async fn set_local_password(
    session: Session,
    Json(request): Json<SetLocalPasswordRequest>,
) -> ApiResult<()> {
    let account_uuid: Uuid = session
        .get("account")
        .await?
        .ok_or(ApiError::bad_request("Not logged-in"))?;

    let mut tx = AuthModule::global().db.start_transaction().await?;

    let _local_uuid = rorm::query(&mut tx, LocalAccount.uuid)
        .condition(LocalAccount.account.equals(&account_uuid))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("User is not a local one"))?;

    // TODO: hashing

    rorm::update(&mut tx, LocalAccount)
        .set(LocalAccount.password, Some(request))
        .condition(LocalAccount.account.equals(&account_uuid))
        .await?;

    tx.commit().await?;

    Ok(())
}

#[delete("/local/password", core_crate = "::galvyn_core")]
pub async fn delete_local_password(session: Session) -> ApiResult<()> {
    let account_uuid: Uuid = session
        .get("account")
        .await?
        .ok_or(ApiError::bad_request("Not logged-in"))?;

    let mut tx = AuthModule::global().db.start_transaction().await?;

    let local_uuid = rorm::query(&mut tx, LocalAccount.uuid)
        .condition(LocalAccount.account.equals(&account_uuid))
        .optional()
        .await?
        .ok_or(ApiError::bad_request("User is not a local one"))?;

    let has_webauthn = rorm::query(&mut tx, WebAuthnKey.key)
        .condition(WebAuthnKey.local_account.equals(&local_uuid))
        .all()
        .await?
        .into_iter()
        .any(|key| matches!(key.0, MaybeAttestedPasskey::Attested(_)));
    if !has_webauthn {
        return Err(ApiError::bad_request("User has no other login method"));
    }

    rorm::update(&mut tx, LocalAccount)
        .set(LocalAccount.password, None)
        .condition(LocalAccount.account.equals(&account_uuid))
        .await?;

    tx.commit().await?;
    Ok(())
}
