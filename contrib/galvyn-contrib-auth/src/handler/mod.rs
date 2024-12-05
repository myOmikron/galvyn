use crate::handler::schema::{
    FinishLoginOidcRequest, GetLoginFlowsRequest, GetLoginFlowsResponse, LocalLoginFlow,
    OidcLoginFlow,
};
use crate::models::AuthModels;
use crate::MaybeAttestedPasskey;
use galvyn_core::re_exports::axum::extract::Query;
use galvyn_core::re_exports::axum::response::Redirect;
use galvyn_core::re_exports::axum::Json;
use galvyn_core::stuff::api_error::{ApiError, ApiResult};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient};
use openidconnect::reqwest::async_http_client;
use openidconnect::{
    AccessTokenHash, CsrfToken, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
    Scope, TokenResponse,
};
use rorm::crud::query::QueryBuilder;
use rorm::internal::field::foreign_model::FieldEq_ForeignModelByField_Borrowed;
use rorm::prelude::ForeignModelByField;
use rorm::Database;
use rorm::FieldAccess;
use std::sync::LazyLock;

mod schema;

static DB: LazyLock<Database> = LazyLock::new(|| todo!());
static OIDC: LazyLock<CoreClient> = LazyLock::new(|| todo!());

pub async fn get_login_flow<M: AuthModels>(
    Json(request): Json<GetLoginFlowsRequest>,
) -> ApiResult<Json<Option<GetLoginFlowsResponse>>> {
    let mut tx = DB.start_transaction().await?;

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

pub async fn login_oidc<M: AuthModels>(session: Session) -> ApiResult<Redirect> {
    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    let request = OIDC
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(pkce_code_challenge)
        .add_scope(Scope::new("profile".to_string())) // TODO: make this configurable
        .add_scope(Scope::new("email".to_string()));
    let (auth_url, csrf_token, nonce) = request.url();

    // TODO: store stuff in session

    Ok(Redirect::temporary(auth_url.as_str()))
}

pub async fn finish_login_oidc<M: AuthModels>(
    session: Session,
    Query(request): Query<FinishLoginOidcRequest>,
) -> ApiResult<Redirect> {
    let csrf_token: CsrfToken = todo!();
    let pkce_code_verifier: PkceCodeVerifier = todo!();
    let nonce: Nonce = todo!();

    if request.state.secret() != csrf_token.secret() {
        return Err("Bad Request".into());
    }

    let token = OIDC
        .exchange_code(request.code)
        .set_pkce_verifier(pkce_code_verifier)
        .request_async(async_http_client)
        .await?;

    let id_token = token.id_token().ok_or_else(|| "Missing id token")?;
    let claims = id_token.claims(&OIDC.id_token_verifier(), &nonce)?;

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

    let mut tx = DB.start_transaction().await?;

    let account_pk = if let Some((account_fm,)) =
        QueryBuilder::new(&mut tx, (M::oidc_account_fm(),))
            .condition(M::oidc_account_id().equals(oidc_id))
            .optional()
            .await?
    {
        match account_fm {
            ForeignModelByField::Key(x) => x,
            ForeignModelByField::Instance(_) => unreachable!(),
        }
    } else {
        // TODO: create account

        // TODO: create oidc account

        todo!()
    };

    // TODO: insert session

    tx.commit().await?;

    Ok(Redirect::temporary("/"))
}

pub async fn login_local_webauthn<M: AuthModels>() {
    todo!()
}

pub async fn finish_login_local_webauthn<M: AuthModels>() {
    todo!()
}

pub async fn login_local_password<M: AuthModels>() {
    todo!()
}

pub async fn logout<M: AuthModels>() {
    todo!()
}
