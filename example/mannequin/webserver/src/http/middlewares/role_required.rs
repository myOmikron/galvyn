//! Authentication required middleware

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::task::Poll;

use axum::extract::FromRequestParts;
use axum::extract::Request;
use axum::response::IntoResponse;
use axum::response::Response;
use tracing::trace;

use crate::http::common::errors::ApiError;
use crate::http::extractors::session_user::SessionUser;
use crate::impl_axum_layer;
use crate::models::UserRole;

/// Middleware which checks the [`SessionUser`] to have a certain [`UserRole`]
#[derive(Copy, Clone, Debug)]
pub struct RoleRequiredLayer {
    role: UserRole,
}
impl RoleRequiredLayer {
    /// Constructs a new `RoleRequiredLayer`
    pub const fn new(required_role: UserRole) -> Self {
        Self {
            role: required_role,
        }
    }
}
impl_axum_layer!(RoleRequiredLayer => RoleRequiredService);
impl RoleRequiredLayer {
    async fn call(self, req: Request) -> ControlFlow<Response, Request> {
        let (mut parts, body) = req.into_parts();
        let user = match SessionUser::from_request_parts(&mut parts, &()).await {
            Ok(user) => user.user,
            Err(error) => return ControlFlow::Break(error.into_response()),
        };

        let user_role = match user.role.key().parse::<UserRole>() {
            Ok(role) => role,
            Err(error) => return ControlFlow::Break(ApiError::from(error).into_response()),
        };

        if user_role == self.role {
            ControlFlow::Continue(Request::from_parts(parts, body))
        } else {
            trace!(
                user = user.display_name,
                user_role = %user_role,
                required_role = %self.role,
                "Missing privileges due to invalid role"
            );
            ControlFlow::Break(ApiError::MissingPrivileges.into_response())
        }
    }
}
