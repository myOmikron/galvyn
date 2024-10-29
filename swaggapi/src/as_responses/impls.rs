use std::borrow::Cow;

use axum::response::{Html, Redirect};
use axum::Json;
use bytes::buf::Chain;
use bytes::Buf;
use bytes::Bytes;
use bytes::BytesMut;
use indexmap::IndexMap;
use openapiv3::Header;
use openapiv3::ParameterSchemaOrContent;
use openapiv3::ReferenceOr;
use openapiv3::Response;
use openapiv3::Responses;
use openapiv3::StatusCode;
use schemars::JsonSchema;
use serde::Serialize;

use crate::as_responses::ok_empty;
use crate::as_responses::ok_json;
use crate::as_responses::ok_text;
use crate::as_responses::AsResponses;
use crate::as_responses::{ok_binary, ok_html};
use crate::internals::SchemaGenerator;

impl AsResponses for &'static str {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_text()
    }
}

impl AsResponses for String {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_text()
    }
}

impl AsResponses for Box<str> {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_text()
    }
}

impl AsResponses for Cow<'static, str> {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_text()
    }
}

impl AsResponses for &'static [u8] {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl<const N: usize> AsResponses for &'static [u8; N] {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl<const N: usize> AsResponses for [u8; N] {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl AsResponses for Vec<u8> {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl AsResponses for Box<[u8]> {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl AsResponses for Bytes {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl AsResponses for BytesMut {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl AsResponses for Cow<'static, [u8]> {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl<T: Serialize + JsonSchema> AsResponses for Json<T> {
    fn responses(gen: &mut SchemaGenerator) -> Responses {
        ok_json::<T>(gen)
    }
}

impl AsResponses for () {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_empty()
    }
}

impl<T, E> AsResponses for Result<T, E>
where
    T: AsResponses,
    E: AsResponses,
{
    fn responses(gen: &mut SchemaGenerator) -> Responses {
        let mut res = E::responses(gen);
        let ok_res = T::responses(gen);

        // As we want to preserve in almost any cases the Ok branch of the result, we're extending
        // the IndexMaps of the error-branch with those of the ok-branch
        res.responses.extend(ok_res.responses);
        res.extensions.extend(ok_res.extensions);
        if ok_res.default.is_some() {
            res.default = ok_res.default;
        }

        res
    }
}

impl AsResponses for Redirect {
    fn responses(gen: &mut SchemaGenerator) -> Responses {
        Responses {
            responses: IndexMap::from_iter([(
                StatusCode::Range(3),
                ReferenceOr::Item(Response {
                    description: "A generic http redirect".to_string(),
                    headers: IndexMap::from_iter([(
                        "Location".to_string(),
                        ReferenceOr::Item(Header {
                            description: None,
                            style: Default::default(),
                            required: false,
                            deprecated: None,
                            format: ParameterSchemaOrContent::Schema(gen.generate::<String>()),
                            example: None,
                            examples: Default::default(),
                            extensions: Default::default(),
                        }),
                    )]),
                    ..Default::default()
                }),
            )]),
            ..Default::default()
        }
    }
}

impl<T, U> AsResponses for Chain<T, U>
where
    T: Buf + Unpin + Send + 'static,
    U: Buf + Unpin + Send + 'static,
{
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_binary()
    }
}

impl<T> AsResponses for Html<T> {
    fn responses(_gen: &mut SchemaGenerator) -> Responses {
        ok_html()
    }
}
