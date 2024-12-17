use crate::handler::schema::{
    FinishLoginOidcRequest, GetLoginFlowsRequest, GetLoginFlowsResponse, LocalLoginFlow,
    LoginLocalPasswordRequest, LoginLocalWebauthnRequest, OidcLoginFlow, PublicKeyCredential,
};
use crate::models::AuthModels;
use crate::module::AuthModule;
use crate::MaybeAttestedPasskey;
use galvyn_core::re_exports::axum::extract::Query;
use galvyn_core::re_exports::axum::response::Redirect;
use galvyn_core::re_exports::axum::Json;
use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::ApiResult;
use galvyn_core::Module;
use galvyn_macros::{get, post};
use openidconnect::core::CoreAuthenticationFlow;
use openidconnect::reqwest::async_http_client;
use openidconnect::{
    AccessTokenHash, CsrfToken, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
    Scope, TokenResponse,
};
use rorm::crud::query::QueryBuilder;
use rorm::internal::field::foreign_model::FieldEq_ForeignModelByField_Borrowed;
use rorm::prelude::ForeignModelByField;
use rorm::{insert, FieldAccess};
use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::{AttestedPasskeyAuthentication, RequestChallengeResponse};

mod schema;

#[get("/login", core_crate = "::galvyn_core")]
pub async fn get_login_flow<M: AuthModels>(
    Query(request): Query<GetLoginFlowsRequest>,
) -> ApiResult<Json<Option<GetLoginFlowsResponse>>> {
    let mut tx = AuthModule::<M>::global().db.start_transaction().await?;

    let Some((user_pk,)) = QueryBuilder::new(&mut tx, (M::account_pk(),))
        .condition(M::account_id().equals(request.identifier.as_str()))
        .optional()
        .await?
    else {
        return Ok(Json(None));
    };

    let oidc = QueryBuilder::new(&mut tx, (M::oidc_account_pk(),))
        .condition(M::oidc_account_fm().equals::<_, FieldEq_ForeignModelByField_Borrowed>(&user_pk))
        .optional()
        .await?;

    let local = QueryBuilder::new(
        &mut tx,
        (M::local_account_pk(), M::local_account_password()),
    )
    .condition(M::local_account_fm().equals::<_, FieldEq_ForeignModelByField_Borrowed>(&user_pk))
    .optional()
    .await?;

    let response = match (oidc, local) {
        (Some(_), None) => GetLoginFlowsResponse::Oidc(OidcLoginFlow {}),
        (None, Some((local_pk, password))) => {
            let webauthn = QueryBuilder::new(&mut tx, (M::webauthn_key_key(),))
                .condition(
                    M::webauthn_key_fm()
                        .equals::<_, FieldEq_ForeignModelByField_Borrowed>(&local_pk),
                )
                .all()
                .await?
                .into_iter()
                .any(|(key,)| matches!(key.0, MaybeAttestedPasskey::Attested(_)));

            GetLoginFlowsResponse::Local(LocalLoginFlow {
                password: password.is_some(),
                webauthn,
            })
        }
        _ => return Err("Invalid account".into()),
    };

    tx.commit().await?;
    Ok(Json(Some(response)))
}

#[post("/login/oidc/start", core_crate = "::galvyn_core")]
pub async fn login_oidc<M: AuthModels>(session: Session) -> ApiResult<Redirect> {
    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    let request = AuthModule::<M>::global()
        .oidc
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(pkce_code_challenge)
        .add_scope(Scope::new("profile".to_string())) // TODO: make this configurable
        .add_scope(Scope::new("email".to_string()));
    let (auth_url, csrf_token, nonce) = request.url();

    session
        .insert(
            "login_oidc",
            LoginOidcSessionData {
                csrf_token,
                pkce_code_verifier,
                nonce,
            },
        )
        .await?;

    Ok(Redirect::temporary(auth_url.as_str()))
}

#[derive(Serialize, Deserialize)]
struct LoginOidcSessionData {
    csrf_token: CsrfToken,
    pkce_code_verifier: PkceCodeVerifier,
    nonce: Nonce,
}

#[post("/login/oidc/finish", core_crate = "::galvyn_core")]
pub async fn finish_login_oidc<M: AuthModels>(
    session: Session,
    Query(request): Query<FinishLoginOidcRequest>,
) -> ApiResult<Redirect> {
    let LoginOidcSessionData {
        csrf_token,
        pkce_code_verifier,
        nonce,
    } = session
        .remove("oidc_login_data")
        .await?
        .ok_or("Bad Request")?;

    if request.state.secret() != csrf_token.secret() {
        return Err("Bad Request".into());
    }

    let token = AuthModule::<M>::global()
        .oidc
        .exchange_code(request.code)
        .set_pkce_verifier(pkce_code_verifier)
        .request_async(async_http_client)
        .await?;

    let id_token = token.id_token().ok_or_else(|| "Missing id token")?;
    let claims = id_token.claims(&AuthModule::<M>::global().oidc.id_token_verifier(), &nonce)?;

    // Verify the access token hash to ensure that the access token hasn't been substituted for
    // another user's.
    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash =
            AccessTokenHash::from_token(token.access_token(), &id_token.signing_alg()?)?;
        if actual_access_token_hash != *expected_access_token_hash {
            return Err("The access token hash is invalid".into());
        }
    }

    // TODO: extract claims
    let Some(oidc_id) = claims.preferred_username().map(|x| x.to_string()) else {
        return Err("Missing claim: preferred_username".into());
    };

    let mut tx = AuthModule::<M>::global().db.start_transaction().await?;

    let account_pk = if let Some((account_fm,)) =
        QueryBuilder::new(&mut tx, (M::oidc_account_fm(),))
            .condition(M::oidc_account_id().equals(&oidc_id))
            .optional()
            .await?
    {
        // TODO: update account with claims

        match account_fm {
            ForeignModelByField::Key(x) => x,
            ForeignModelByField::Instance(_) => unreachable!(),
        }
    } else {
        // TODO: create account with claims

        let account_pk = insert!(&mut tx, M::Account)
            .return_primary_key()
            .single(&M::insertable_account(oidc_id.clone()))
            .await?;

        insert!(&mut tx, M::OidcAccount)
            .return_nothing()
            .single(&M::insertable_oidc_account(oidc_id, &account_pk))
            .await?;

        account_pk
    };

    tx.commit().await?;

    session.insert("account", account_pk).await?;

    Ok(Redirect::temporary("/"))
}

#[post("/login/local/start-webauthn", core_crate = "::galvyn_core")]
pub async fn login_local_webauthn<M: AuthModels>(
    session: Session,
    Json(request): Json<LoginLocalWebauthnRequest>,
) -> ApiResult<Json<RequestChallengeResponse>> {
    let mut tx = AuthModule::<M>::global().db.start_transaction().await?;

    let (account_pk,) = QueryBuilder::new(&mut tx, (M::account_pk(),))
        .condition(M::account_id().equals(&request.identifier))
        .optional()
        .await?
        .ok_or("Account not found")?;

    let (local_account_pk,) = QueryBuilder::new(&mut tx, (M::local_account_pk(),))
        .condition(
            M::local_account_fm().equals::<_, FieldEq_ForeignModelByField_Borrowed>(&account_pk),
        )
        .optional()
        .await?
        .ok_or("Not a local account")?;

    let keys = QueryBuilder::new(&mut tx, (M::webauthn_key_key(),))
        .condition(
            M::webauthn_key_fm()
                .equals::<_, FieldEq_ForeignModelByField_Borrowed>(&local_account_pk),
        )
        .all()
        .await?;
    let keys = keys
        .into_iter()
        .filter_map(|(json,)| match json.0 {
            MaybeAttestedPasskey::NotAttested(_) => None,
            MaybeAttestedPasskey::Attested(key) => Some(key),
        })
        .collect::<Vec<_>>();

    let (challenge, state) = AuthModule::<M>::global()
        .webauthn
        .start_attested_passkey_authentication(&keys)?;

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

    Ok(Json(challenge))
}

#[derive(Serialize, Deserialize)]
struct LoginLocalWebauthnSessionData {
    identifier: String,
    state: AttestedPasskeyAuthentication,
}

#[post("/login/local/finish-webauthn", core_crate = "::galvyn_core")]
pub async fn finish_login_local_webauthn<M: AuthModels>(
    session: Session,
    Json(request): Json<PublicKeyCredential>,
) -> ApiResult<()> {
    let LoginLocalWebauthnSessionData { identifier, state } = session
        .remove("login_local_webauthn")
        .await?
        .ok_or("Bad Request")?;

    let authentication_result = AuthModule::<M>::global()
        .webauthn
        .finish_attested_passkey_authentication(&request.0, &state)?;

    let mut tx = AuthModule::<M>::global().db.start_transaction().await?;

    let (account_pk,) = QueryBuilder::new(&mut tx, (M::account_pk(),))
        .condition(M::account_id().equals(&identifier))
        .optional()
        .await?
        .ok_or("Account not found")?;

    let (local_account_pk,) = QueryBuilder::new(&mut tx, (M::local_account_pk(),))
        .condition(
            M::local_account_fm().equals::<_, FieldEq_ForeignModelByField_Borrowed>(&account_pk),
        )
        .optional()
        .await?
        .ok_or("Not a local account")?;

    let keys = QueryBuilder::new(&mut tx, (M::webauthn_key_key(),))
        .condition(
            M::webauthn_key_fm()
                .equals::<_, FieldEq_ForeignModelByField_Borrowed>(&local_account_pk),
        )
        .all()
        .await?;
    let _used_key = keys
        .into_iter()
        .find_map(|(json,)| match json.0 {
            MaybeAttestedPasskey::NotAttested(_) => None,
            MaybeAttestedPasskey::Attested(key) => {
                (key.cred_id() == authentication_result.cred_id()).then_some(key)
            }
        })
        .ok_or("Used unknown key")?;

    tx.commit().await?;

    session.insert("account", account_pk).await?;

    Ok(())
}

#[post("/login/local/password", core_crate = "::galvyn_core")]
pub async fn login_local_password<M: AuthModels>(
    session: Session,
    Json(request): Json<LoginLocalPasswordRequest>,
) -> ApiResult<()> {
    let mut tx = AuthModule::<M>::global().db.start_transaction().await?;

    let (account_pk,) = QueryBuilder::new(&mut tx, (M::account_pk(),))
        .condition(M::account_id().equals(&request.identifier))
        .optional()
        .await?
        .ok_or("Account not found")?;

    let (local_account_password,) = QueryBuilder::new(&mut tx, (M::local_account_password(),))
        .condition(
            M::local_account_fm().equals::<_, FieldEq_ForeignModelByField_Borrowed>(&account_pk),
        )
        .optional()
        .await?
        .ok_or("Not a local account")?;

    let local_account_password = local_account_password.ok_or("Account has no password")?;
    // TODO: hashing
    if local_account_password != request.password {
        return Err("Passwords do not match".into());
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
