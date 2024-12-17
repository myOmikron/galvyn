use crate::{AuthModels, AuthModule, MaybeAttestedPasskey};
use galvyn_core::re_exports::axum::Json;
use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::ApiResult;
use galvyn_core::Module;
use galvyn_macros::{delete, put};
use rorm::crud::query::QueryBuilder;
use rorm::crud::update::UpdateBuilder;
use rorm::internal::field::Field;
use rorm::{FieldAccess, Model};

type SetLocalPasswordRequest = String;

#[put("/local/password", core_crate = "::galvyn_core")]
pub async fn set_local_password<M: AuthModels>(
    session: Session,
    Json(request): Json<SetLocalPasswordRequest>,
) -> ApiResult<()> {
    let account_pk: <<M::Account as Model>::Primary as Field>::Type =
        session.get("account").await?.ok_or("Not logged-in")?;

    let mut tx = AuthModule::<M>::global().db.start_transaction().await?;

    let (_local_pk,) = QueryBuilder::new(&mut tx, (M::local_account_pk(),))
        .condition(M::local_account_fm().equals(&account_pk))
        .optional()
        .await?
        .ok_or("User is not a local one")?;

    // TODO: hashing

    UpdateBuilder::new(&mut tx)
        .condition(M::local_account_fm().equals(&account_pk))
        .set(M::local_account_password(), Some(request))
        .exec()
        .await?;

    tx.commit().await?;

    Ok(())
}

#[delete("/local/password", core_crate = "::galvyn_core")]
pub async fn delete_local_password<M: AuthModels>(session: Session) -> ApiResult<()> {
    let account_pk: <<M::Account as Model>::Primary as Field>::Type =
        session.get("account").await?.ok_or("Not logged-in")?;

    let mut tx = AuthModule::<M>::global().db.start_transaction().await?;

    let (local_pk,) = QueryBuilder::new(&mut tx, (M::local_account_pk(),))
        .condition(M::local_account_fm().equals(&account_pk))
        .optional()
        .await?
        .ok_or("User is not a local one")?;

    let has_webauthn = QueryBuilder::new(&mut tx, (M::webauthn_key_key(),))
        .condition(M::webauthn_key_fm().equals(&local_pk))
        .all()
        .await?
        .into_iter()
        .any(|(key,)| matches!(key.0, MaybeAttestedPasskey::Attested(_)));
    if !has_webauthn {
        return Err("User has no other login method".into());
    }

    UpdateBuilder::new(&mut tx)
        .condition(M::local_account_fm().equals(&account_pk))
        .set(M::local_account_password(), None)
        .exec()
        .await?;

    tx.commit().await?;
    Ok(())
}
