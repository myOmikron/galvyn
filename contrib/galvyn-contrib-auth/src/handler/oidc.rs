use crate::handler::schema::FinishLoginOidcRequest;
use crate::{AuthModels, AuthModule};
use galvyn_core::re_exports::axum::extract::Query;
use galvyn_core::re_exports::axum::response::Redirect;
use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::ApiResult;
use galvyn_core::Module;
use galvyn_macros::post;
use openidconnect::core::CoreAuthenticationFlow;
use openidconnect::reqwest::async_http_client;
use openidconnect::OAuth2TokenResponse;
use openidconnect::TokenResponse;
use openidconnect::{
    AccessTokenHash, CsrfToken, Nonce, PkceCodeChallenge, PkceCodeVerifier, Scope,
};
use rorm::crud::query::QueryBuilder;
use rorm::insert;
use rorm::prelude::ForeignModelByField;
use rorm::FieldAccess;
use serde::{Deserialize, Serialize};

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
