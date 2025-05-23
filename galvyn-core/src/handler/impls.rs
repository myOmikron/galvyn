use std::any::type_name;
use std::borrow::Cow;

use axum::Form;
use axum::Json;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::RawForm;
use axum::http::HeaderName;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::Html;
use axum::response::Redirect;
use bytes::Buf;
use bytes::BytesMut;
use bytes::buf::Chain;
use mime::Mime;
use schemars::JsonSchema;
use schemars::schema::Schema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::debug;
use tracing::warn;

use super::request_body::RequestBody;
use super::request_body::ShouldBeRequestBody;
use super::request_part::RequestPart;
use super::request_part::ShouldBeRequestPart;
use crate::handler::response_body::ResponseBody;
use crate::handler::response_body::ShouldBeResponseBody;
use crate::schema_generator::SchemaGenerator;

impl ShouldBeRequestBody for String {}
impl RequestBody for String {
    fn body(_generator: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
        (mime::TEXT_PLAIN_UTF_8, None)
    }
}

impl ShouldBeRequestBody for Bytes {}
impl RequestBody for Bytes {
    fn body(_generator: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
        (mime::APPLICATION_OCTET_STREAM, None)
    }
}

impl<T> ShouldBeRequestBody for Json<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestBody for Json<T> {
    fn body(generator: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
        (mime::APPLICATION_JSON, Some(generator.generate::<T>()))
    }
}

impl<T> ShouldBeRequestBody for Form<T> {}
/*
impl<T: DeserializeOwned + JsonSchema> HandlerArgument for Form<T> {
    fn request_body(generator: &mut SchemaGenerator) -> Option<RequestBody> {
        let schema = convert_schema(gen.generate::<T>());
        Some(simple_request_body(SimpleRequestBody {
            mime_type: mime::APPLICATION_WWW_FORM_URLENCODED,
            schema: Some(schema),
        }))
    }
}
*/

impl ShouldBeRequestBody for RawForm {}
/*
impl HandlerArgument for RawForm {
    fn request_body(_generator: &mut SchemaGenerator) -> Option<RequestBody> {
        Some(simple_request_body(SimpleRequestBody {
            mime_type: mime::APPLICATION_WWW_FORM_URLENCODED,
            schema: None,
        }))
    }
}
*/
impl<T> ShouldBeRequestPart for Path<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestPart for Path<T> {
    fn path_parameters(generator: &mut SchemaGenerator) -> Vec<(String, Option<Schema>)> {
        let Some(obj) = generator.generate_object::<T>() else {
            warn!("Unsupported handler argument: {}", type_name::<Self>());
            debug!("generate_object::<{}>() == None", type_name::<T>());
            return Vec::new();
        };

        obj.properties
            .into_iter()
            .map(|(name, schema)| (name, Some(schema)))
            .collect()
    }
}

impl<T> ShouldBeRequestPart for Query<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestPart for Query<T> {
    fn query_parameters(generator: &mut SchemaGenerator) -> Vec<(String, Option<Schema>)> {
        let Some(obj) = generator.generate_object::<T>() else {
            warn!("Unsupported handler argument: {}", type_name::<Self>());
            return Vec::new();
        };

        obj.properties
            .into_iter()
            .map(|(name, schema)| (name, Some(schema)))
            .collect()
    }
}

impl ShouldBeResponseBody for &'static str {}
impl ResponseBody for &'static str {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for String {}
impl ResponseBody for String {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for Box<str> {}
impl ResponseBody for Box<str> {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for Cow<'static, str> {}
impl ResponseBody for Cow<'static, str> {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for &'static [u8] {}
impl ResponseBody for &'static [u8] {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<const N: usize> ShouldBeResponseBody for &'static [u8; N] {}
impl<const N: usize> ResponseBody for &'static [u8; N] {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<const N: usize> ShouldBeResponseBody for [u8; N] {}
impl<const N: usize> ResponseBody for [u8; N] {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Vec<u8> {}
impl ResponseBody for Vec<u8> {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Box<[u8]> {}
impl ResponseBody for Box<[u8]> {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Bytes {}
impl ResponseBody for Bytes {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for BytesMut {}
impl ResponseBody for BytesMut {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Cow<'static, [u8]> {}
impl ResponseBody for Cow<'static, [u8]> {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<T> ShouldBeResponseBody for Json<T> {}
impl<T: Serialize + JsonSchema> ResponseBody for Json<T> {
    fn body(generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(
            StatusCode::OK,
            Some((mime::APPLICATION_JSON, Some(generator.generate::<T>()))),
        )]
    }
}

impl ShouldBeResponseBody for () {}
impl ResponseBody for () {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, None)]
    }
}

impl<T, E> ShouldBeResponseBody for Result<T, E>
where
    T: ShouldBeResponseBody, // TODO: find better solution / compromise
    E: ShouldBeResponseBody, //       ideally Result<T, E>: ShouldBeResponseBody
                             //       if either T or E are ShouldBeResponseBody
{
}
impl<T, E> ResponseBody for Result<T, E>
where
    T: ResponseBody,
    E: ResponseBody,
{
    fn body(generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        let mut bodies = T::body(&mut *generator);
        bodies.extend(E::body(&mut *generator));
        bodies
    }
}

impl ShouldBeResponseBody for Redirect {}
impl ResponseBody for Redirect {
    fn header() -> Vec<HeaderName> {
        vec![header::LOCATION]
    }

    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![
            (StatusCode::SEE_OTHER, None),
            (StatusCode::TEMPORARY_REDIRECT, None),
            (StatusCode::PERMANENT_REDIRECT, None),
        ]
    }
    // fn responses(generator: &mut SchemaGenerator) -> Responses {
    //     Responses {
    //         responses: IndexMap::from_iter([(
    //             StatusCode::Range(3),
    //             ReferenceOr::Item(Response {
    //                 description: "A generic http redirect".to_string(),
    //                 headers: IndexMap::from_iter([(
    //                     "Location".to_string(),
    //                     ReferenceOr::Item(Header {
    //                         description: None,
    //                         style: Default::default(),
    //                         required: false,
    //                         deprecated: None,
    //                         format: ParameterSchemaOrContent::Schema(gen.generate::<String>()),
    //                         example: None,
    //                         examples: Default::default(),
    //                         extensions: Default::default(),
    //                     }),
    //                 )]),
    //                 ..Default::default()
    //             }),
    //         )]),
    //         ..Default::default()
    //     }
    // }
}

impl<T, U> ShouldBeResponseBody for Chain<T, U>
where
    T: Buf + Unpin + Send + 'static,
    U: Buf + Unpin + Send + 'static,
{
}
impl<T, U> ResponseBody for Chain<T, U>
where
    T: Buf + Unpin + Send + 'static,
    U: Buf + Unpin + Send + 'static,
{
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<T> ShouldBeResponseBody for Html<T> {}
impl<T> ResponseBody for Html<T> {
    fn body(_generator: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_HTML_UTF_8, None)))]
    }
}
