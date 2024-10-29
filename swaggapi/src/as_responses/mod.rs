//! The [`AsResponses`] trait, its implementations and utilises for implementing it.

use indexmap::IndexMap;
use mime::Mime;
use openapiv3::MediaType;
use openapiv3::ReferenceOr;
use openapiv3::Response;
use openapiv3::Responses;
use openapiv3::StatusCode;
use schemars::JsonSchema;

use crate::internals::SchemaGenerator;

mod impls;
mod utils;

/// A type returned by a handler which can be described with a [responses object](https://spec.openapis.org/oas/v3.0.3#responses-object)
///
/// This type should be implemented by everything which implements
/// [`IntoResponse`](::axum::response::IntoResponse) when using [axum](::axum) or
/// [`Responder`] when using [actix]
pub trait AsResponses {
    /// Get the [responses object](https://spec.openapis.org/oas/v3.0.3#responses-object) describing `Self`
    fn responses(gen: &mut SchemaGenerator) -> Responses;
}

/// Helper function for building a [`Responses`] for some simple responses
pub fn simple_responses(responses: impl IntoIterator<Item = SimpleResponse>) -> Responses {
    Responses {
        responses: responses
            .into_iter()
            .map(|response: SimpleResponse| {
                (
                    response.status_code,
                    ReferenceOr::Item(Response {
                        description: response.description,
                        content: FromIterator::from_iter([(
                            response.mime_type.to_string(),
                            response.media_type.unwrap_or_default(),
                        )]),
                        ..Default::default()
                    }),
                )
            })
            .collect(),
        ..Default::default()
    }
}

/// Describes the response for a specific status code
pub struct SimpleResponse {
    /// The response's status code
    pub status_code: StatusCode,

    /// The response's mime type
    pub mime_type: Mime,

    /// A short description of the response.
    /// CommonMark syntax MAY be used for rich text representation.
    pub description: String,

    /// Optional more details explanation of the response's data
    pub media_type: Option<MediaType>,
}

/// Helper function for building a [`Responses`] for a simple `200` plain text response
pub fn ok_text() -> Responses {
    simple_responses([SimpleResponse {
        status_code: StatusCode::Code(200),
        mime_type: mime::TEXT_PLAIN_UTF_8,
        description: "Some plain text".to_string(),
        media_type: None,
    }])
}

/// Helper function for building a [`Responses`] for a simple `200` html utf-8 response
pub fn ok_html() -> Responses {
    simple_responses([SimpleResponse {
        status_code: StatusCode::Code(200),
        mime_type: mime::TEXT_HTML_UTF_8,
        description: "Some html".to_string(),
        media_type: None,
    }])
}

/// Helper function for building a [`Responses`] for a simple `200` binary response
pub fn ok_binary() -> Responses {
    simple_responses([SimpleResponse {
        status_code: StatusCode::Code(200),
        mime_type: mime::APPLICATION_OCTET_STREAM,
        description: "Some binary data".to_string(),
        media_type: None,
    }])
}

/// Helper function for building a [`Responses`]  for a simple `200` empty response
pub fn ok_empty() -> Responses {
    Responses {
        responses: IndexMap::from([(
            StatusCode::Code(200),
            ReferenceOr::Item(Response {
                description: "Empty body".to_string(),
                ..Default::default()
            }),
        )]),
        ..Default::default()
    }
}

/// Helper function for building a [`Responses`] for a simple `200` json response using a schema
pub fn ok_json<T: JsonSchema>(gen: &mut SchemaGenerator) -> Responses {
    simple_responses([
        SimpleResponse {
            status_code: StatusCode::Code(200),
            mime_type: mime::APPLICATION_JSON,
            description: "".to_string(), // TODO take the schema's
            media_type: Some(MediaType {
                schema: Some(gen.generate::<T>()),
                ..Default::default()
            }),
        },
        // TODO add error
    ])
}
