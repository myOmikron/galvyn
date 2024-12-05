use crate::global::GLOBAL;
use crate::http::common::errors::ApiResult;
use crate::http::handler_frontend::user_invites::schema::SimpleUserInvite;
use crate::models::UserInvite;
use crate::utils::checked_string::CheckedString;
use crate::utils::links::new_user_invite_link;
use crate::utils::schemars::SchemaDateTime;

pub fn new_simple_user_invite(invite: UserInvite) -> ApiResult<SimpleUserInvite> {
    Ok(SimpleUserInvite {
        uuid: invite.uuid,
        link: new_user_invite_link(&GLOBAL.origin, invite.uuid),
        mail: CheckedString::new(invite.email)?,
        display_name: CheckedString::new(invite.display_name)?,
        preferred_lang: invite.preferred_lang.parse()?,
        permissions: invite.permissions.0,
        expires_at: SchemaDateTime(invite.expires_at),
        created_at: SchemaDateTime(invite.created_at),
    })
}
