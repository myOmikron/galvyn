use galvyn_core::Module;
use galvyn_core::re_exports::axum::extract::Path;
use galvyn_core::re_exports::axum::extract::Query;
use galvyn_core::re_exports::axum::response::Redirect;
use galvyn_core::re_exports::uuid::Uuid;
use galvyn_core::stuff::api_error::ApiError;
use galvyn_core::stuff::api_error::ApiResult;
use galvyn_core::stuff::schema::SingleUuid;
use galvyn_macros::get;
use tracing::info;
use url::Url;

use crate::OauthProviderModule;
use crate::handler::error::OauthErrorBuilder;
use crate::handler::error::OauthResult;
use crate::handler::schema::AuthErrorType;
use crate::handler::schema::AuthRequest;
use crate::handler::schema::CodeChallengeMethod;
use crate::models::GalvynOauthClient;
use crate::module::OauthRequest;

mod error;
mod schema;

/// Initial endpoint an application redirects the user to.
///
/// It requires both the `state` parameter against CSRF, as well as a pkce challenge.
/// The only supported pkce `code_challenge_method` is `S256`.
#[get("/auth", core_crate = "::galvyn_core")]
pub async fn auth(Query(request): Query<AuthRequest>) -> OauthResult<Redirect> {
    let error_builder = OauthErrorBuilder::from_request(&request)?;

    if request.response_type != "code" {
        return Err(error_builder.new_error(
            AuthErrorType::UnsupportedResponseType,
            "Only supported response_type is code",
        ));
    }

    let client_uuid = match Uuid::parse_str(request.client_id.as_str()) {
        Ok(x) => x,
        Err(error) => {
            info!(
                request.client_id = request.client_id,
                error.display = %error,
                error.debug = ?error,
                "Client ids are always uuids"
            );
            return Err(error_builder.new_error(AuthErrorType::InvalidRequest, "Invalid client id"));
        }
    };

    // TODO: Parse and validate scope

    let Some(state) = request.state else {
        return Err(error_builder.new_error(AuthErrorType::InvalidRequest, "Missing state"));
    };

    let Some(code_challenge) = request.code_challenge else {
        return Err(
            error_builder.new_error(AuthErrorType::InvalidRequest, "Missing code_challenge")
        );
    };

    if request.code_challenge_method != CodeChallengeMethod::Sha256 {
        info!(
            ?request.code_challenge_method,
            "Currently, only sha256 is supported. This could be extended if the need arises."
        );
        return Err(error_builder.new_error(
            AuthErrorType::InvalidRequest,
            "Unsupported code_challenge_method",
        ));
    }

    let mut tx = OauthProviderModule::global()
        .db
        .start_transaction()
        .await
        .map_err(error_builder.map_rorm_error())?;

    let Some(client) = rorm::query(&mut tx, GalvynOauthClient)
        .condition(GalvynOauthClient.uuid.equals(client_uuid))
        .optional()
        .await
        .map_err(error_builder.map_rorm_error())?
    else {
        return Err(error_builder.new_error(AuthErrorType::InvalidRequest, "Invalid client id"));
    };

    if let Some(redirect_uri) = request.redirect_uri.as_deref() {
        if *redirect_uri != *client.redirect_uri {
            info!(
                request.redirect_uri = redirect_uri,
                client.redirect_uri = &*client.redirect_uri,
                "The request's redirect_uri must match the client's redirect_uri exactly!"
            );
            return Err(
                error_builder.new_error(AuthErrorType::InvalidRequest, "Invalid redirect_uri")
            );
        }
    }

    let request_uuid = OauthProviderModule::global().insert_open(OauthRequest {
        client_uuid,
        state,
        scope: (),
        code_challenge,
    });
    let frontend_redirect = OauthProviderModule::global()
        .setup
        .frontend_redirect
        .redirect_uri(request_uuid);

    tx.commit().await.map_err(error_builder.map_rorm_error())?;
    Ok(Redirect::temporary(&frontend_redirect))
}

/// Endpoint visited by user to grant a requesting application access
#[get("/accept/{uuid}", core_crate = "::galvyn_core")]
pub async fn accept(path: Path<SingleUuid>) -> ApiResult<Redirect> {
    let open_request = OauthProviderModule::global()
        .remove_open(path.uuid)
        .ok_or(ApiError::bad_request("Invalid oauth request uuid"))?;
    let response_uuid = OauthProviderModule::global().insert_accepted(open_request.clone());

    let redirect_uri = rorm::query(
        &OauthProviderModule::global().db,
        GalvynOauthClient.redirect_uri,
    )
    .condition(GalvynOauthClient.uuid.equals(open_request.client_uuid))
    .one()
    .await?;

    let mut redirect_uri = Url::parse(&redirect_uri).map_err(ApiError::map_server_error(
        "Invalid redirect uri stored in db",
    ))?;

    {
        let mut query = redirect_uri.query_pairs_mut();
        query.append_pair("code", &response_uuid.to_string());
        query.append_pair("state", &open_request.state);
    }

    Ok(Redirect::temporary(redirect_uri.as_str()))
}

/// Endpoint visited by user to deny a requesting application access
#[get("/deny/{uuid}", core_crate = "::galvyn_core")]
pub async fn deny(path: Path<SingleUuid>) -> ApiResult<Redirect> {
    let open_request = OauthProviderModule::global()
        .remove_open(path.uuid)
        .ok_or(ApiError::bad_request("Invalid oauth request uuid"))?;

    let redirect_uri = rorm::query(
        &OauthProviderModule::global().db,
        GalvynOauthClient.redirect_uri,
    )
    .condition(GalvynOauthClient.uuid.equals(open_request.client_uuid))
    .one()
    .await?;

    let mut redirect_uri = Url::parse(&redirect_uri).map_err(ApiError::map_server_error(
        "Invalid redirect uri stored in db",
    ))?;

    {
        let mut query = redirect_uri.query_pairs_mut();
        query.append_pair("error", "access_denied");
        query.append_pair("state", &open_request.state);
    }

    Ok(Redirect::temporary(redirect_uri.as_str()))
}
