use crate::models::{LocalAccount, WebAuthnKey};
use crate::{AuthModule, MaybeAttestedPasskey};
use galvyn_core::re_exports::axum::Json;
use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::ApiResult;
use galvyn_core::Module;
use galvyn_macros::{delete, put};

type SetLocalPasswordRequest = String;

#[put("/local/password", core_crate = "::galvyn_core")]
pub async fn set_local_password(
    session: Session,
    Json(request): Json<SetLocalPasswordRequest>,
) -> ApiResult<()> {
    let account_pk: i64 = session.get("account").await?.ok_or("Not logged-in")?;

    let mut tx = AuthModule::global().db.start_transaction().await?;

    let _local_pk = rorm::query(&mut tx, LocalAccount.pk)
        .condition(LocalAccount.account.equals(&account_pk))
        .optional()
        .await?
        .ok_or("User is not a local one")?;

    // TODO: hashing

    rorm::update(&mut tx, LocalAccount)
        .set(LocalAccount.password, Some(request))
        .condition(LocalAccount.account.equals(&account_pk))
        .await?;

    tx.commit().await?;

    Ok(())
}

#[delete("/local/password", core_crate = "::galvyn_core")]
pub async fn delete_local_password(session: Session) -> ApiResult<()> {
    let account_pk: i64 = session.get("account").await?.ok_or("Not logged-in")?;

    let mut tx = AuthModule::global().db.start_transaction().await?;

    let local_pk = rorm::query(&mut tx, LocalAccount.pk)
        .condition(LocalAccount.account.equals(&account_pk))
        .optional()
        .await?
        .ok_or("User is not a local one")?;

    let has_webauthn = rorm::query(&mut tx, WebAuthnKey.key)
        .condition(WebAuthnKey.local_account.equals(&local_pk))
        .all()
        .await?
        .into_iter()
        .any(|key| matches!(key.0, MaybeAttestedPasskey::Attested(_)));
    if !has_webauthn {
        return Err("User has no other login method".into());
    }

    rorm::update(&mut tx, LocalAccount)
        .set(LocalAccount.password, None)
        .condition(LocalAccount.account.equals(&account_pk))
        .await?;

    tx.commit().await?;
    Ok(())
}
