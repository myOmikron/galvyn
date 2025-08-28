use schemars::schema::Schema;

use crate::handler::context::EndpointContext;
use crate::macro_utils::type_metadata::HasMetadata;
use crate::macro_utils::type_metadata::ShouldHaveMetadata;

/// Describes the behaviour of a type implementing [`FromRequestParts`](axum::extract::FromRequestParts)
pub trait RequestPart: ShouldBeRequestPart {
    fn query_parameters(_generator: &mut EndpointContext) -> Vec<(String, Option<Schema>)> {
        vec![]
    }

    fn path_parameters(_generator: &mut EndpointContext) -> Vec<(String, Option<Schema>)> {
        vec![]
    }
}

pub trait ShouldBeRequestPart {}

#[derive(Clone, Debug)]
pub struct RequestPartMetadata {
    pub query_parameters: fn(&mut EndpointContext) -> Vec<(String, Option<Schema>)>,
    pub path_parameters: fn(&mut EndpointContext) -> Vec<(String, Option<Schema>)>,
}

impl<T: ShouldBeRequestPart> ShouldHaveMetadata<RequestPartMetadata> for T {}
impl<T: RequestPart> HasMetadata<RequestPartMetadata> for T {
    fn metadata() -> RequestPartMetadata {
        RequestPartMetadata {
            query_parameters: T::query_parameters,
            path_parameters: T::path_parameters,
        }
    }
}
