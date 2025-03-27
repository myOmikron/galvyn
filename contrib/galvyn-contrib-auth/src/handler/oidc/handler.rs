use crate::handler::oidc::schema::FinishLoginOidcRequest;
use crate::models::OidcAccount;
use crate::{Account, AuthModule};
use galvyn_core::re_exports::axum::extract::Query;
use galvyn_core::re_exports::axum::response::Redirect;
use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::{ApiError, ApiResult};
use galvyn_core::Module;
use galvyn_macros::post;
use rorm::and;
use rorm::fields::types::MaxStr;
use rorm::prelude::ForeignModelByField;
use uuid::Uuid;

#[post("/login/oidc/start", core_crate = "::galvyn_core")]
pub async fn login_oidc(session: Session) -> ApiResult<Redirect> {
    let (url, session_state) = AuthModule::global().oidc.begin_login()?;

    session.insert("login_oidc", session_state).await?;

    Ok(Redirect::temporary(url.as_str()))
}

#[post("/login/oidc/finish", core_crate = "::galvyn_core")]
pub async fn finish_login_oidc(
    session: Session,
    Query(request): Query<FinishLoginOidcRequest>,
) -> ApiResult<Redirect> {
    let session_state = session
        .remove("oidc_login_data")
        .await?
        .ok_or(ApiError::bad_request("No ongoing challenge"))?;

    let claims = AuthModule::global()
        .oidc
        .finish_login(session_state, request.0)
        .await?;

    let issuer = MaxStr::new(claims.issuer().to_string())
        .map_err(ApiError::map_server_error("Issuer is too long"))?;

    let subject = MaxStr::new(claims.subject().to_string())
        .map_err(ApiError::map_server_error("Subject is too long"))?;

    // TODO: extract claims

    let mut tx = AuthModule::global().db.start_transaction().await?;

    let existing_account = rorm::query(&mut tx, OidcAccount.account)
        .condition(and![
            OidcAccount.issuer.equals(&*issuer),
            OidcAccount.subject.equals(&*subject)
        ])
        .optional()
        .await?;
    let account_pk = if let Some(account_fm) = existing_account {
        // TODO: update account with claims

        account_fm.0
    } else {
        // TODO: create account with claims

        let account_pk = rorm::insert(&mut tx, Account)
            .return_primary_key()
            .single(&Account {
                uuid: Uuid::new_v4(),
                id: "".to_string(), // TODO
            })
            .await?;

        rorm::insert(&mut tx, OidcAccount)
            .return_nothing()
            .single(&OidcAccount {
                uuid: Uuid::new_v4(),
                issuer,
                subject,
                account: ForeignModelByField(account_pk),
            })
            .await?;

        account_pk
    };

    tx.commit().await?;

    session.insert("account", account_pk).await?;

    Ok(Redirect::temporary("/"))
}
