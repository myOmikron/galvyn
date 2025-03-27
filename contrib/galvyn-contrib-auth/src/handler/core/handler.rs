use galvyn_core::session::Session;
use galvyn_core::stuff::api_error::ApiResult;
use galvyn_macros::post;

#[post("/logout", core_crate = "::galvyn_core")]
pub async fn logout(session: Session) -> ApiResult<()> {
    session.remove::<serde::de::IgnoredAny>("account").await?;
    Ok(())
}
