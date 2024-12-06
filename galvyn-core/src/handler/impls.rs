use super::request_body::{RequestBody, ShouldBeRequestBody};
use super::request_part::{RequestPart, ShouldBeRequestPart};
use crate::handler::response_body::{ResponseBody, ShouldBeResponseBody};
use crate::schema_generator::SchemaGenerator;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::RawForm;
use axum::http::{header, HeaderName, StatusCode};
use axum::response::{Html, Redirect};
use axum::Form;
use axum::Json;
use bytes::buf::Chain;
use bytes::{Buf, BytesMut};
use mime::Mime;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::borrow::Cow;

impl ShouldBeRequestBody for String {}
impl RequestBody for String {
    fn body(_gen: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
        (mime::TEXT_PLAIN_UTF_8, None)
    }
}

impl ShouldBeRequestBody for Bytes {}
impl RequestBody for Bytes {
    fn body(_gen: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
        (mime::APPLICATION_OCTET_STREAM, None)
    }
}

impl<T> ShouldBeRequestBody for Json<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestBody for Json<T> {
    fn body(gen: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
        (mime::APPLICATION_JSON, Some(gen.generate::<T>()))
    }
}

impl<T> ShouldBeRequestBody for Form<T> {}
/*
impl<T: DeserializeOwned + JsonSchema> HandlerArgument for Form<T> {
    fn request_body(gen: &mut SchemaGenerator) -> Option<RequestBody> {
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
    fn request_body(_gen: &mut SchemaGenerator) -> Option<RequestBody> {
        Some(simple_request_body(SimpleRequestBody {
            mime_type: mime::APPLICATION_WWW_FORM_URLENCODED,
            schema: None,
        }))
    }
}
*/
impl<T> ShouldBeRequestPart for Path<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestPart for Path<T> {
    // fn parameters(gen: &mut SchemaGenerator, path: &[&str]) -> Vec<Parameter> {
    //     let Ok(schema) = gen.generate_refless::<T>() else {
    //         warn!("Unsupported handler argument: {}", type_name::<Self>());
    //         debug!("generate_refless::<{}>() == Err(_)", type_name::<T>());
    //         return Vec::new();
    //     };
    //
    //     match schema.schema_kind {
    //         SchemaKind::Type(Type::Object(obj)) => obj
    //             .properties
    //             .into_iter()
    //             .map(|(name, schema)| Parameter::Path {
    //                 parameter_data: ParameterData {
    //                     required: obj.required.contains(&name),
    //                     name,
    //                     description: None,
    //                     deprecated: None,
    //                     format: ParameterSchemaOrContent::Schema(schema.unbox()),
    //                     example: None,
    //                     examples: Default::default(),
    //                     explode: None,
    //                     extensions: Default::default(),
    //                 },
    //                 style: Default::default(),
    //             })
    //             .collect(),
    //         _ if path.len() == 1 => {
    //             vec![Parameter::Path {
    //                 parameter_data: ParameterData {
    //                     name: path[0].to_string(),
    //                     description: None,
    //                     required: !schema.schema_data.nullable,
    //                     deprecated: None,
    //                     format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)),
    //                     example: None,
    //                     examples: Default::default(),
    //                     explode: None,
    //                     extensions: Default::default(),
    //                 },
    //                 style: Default::default(),
    //             }]
    //         }
    //         _ => {
    //             warn!("Unsupported handler argument: {}", type_name::<Self>());
    //             debug!(
    //                 "generate_refless::<{}>() == Ok({schema:#?})",
    //                 type_name::<T>()
    //             );
    //             Vec::new()
    //         }
    //     }
    // }
}

impl<T> ShouldBeRequestPart for Query<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestPart for Query<T> {
    // fn parameters(gen: &mut SchemaGenerator, _path: &[&str]) -> Vec<Parameter> {
    //     let Some((obj, _)) = gen.generate_object::<T>() else {
    //         warn!("Unsupported handler argument: {}", type_name::<Self>());
    //         return Vec::new();
    //     };
    //
    //     obj.properties
    //         .into_iter()
    //         .map(|(name, schema)| Parameter::Query {
    //             parameter_data: ParameterData {
    //                 required: obj.required.contains(&name),
    //                 name,
    //                 description: None,
    //                 deprecated: None,
    //                 format: ParameterSchemaOrContent::Schema(schema.unbox()),
    //                 example: None,
    //                 examples: Default::default(),
    //                 explode: None,
    //                 extensions: Default::default(),
    //             },
    //             allow_reserved: false,
    //             style: Default::default(),
    //             allow_empty_value: None,
    //         })
    //         .collect()
    // }
}

impl ShouldBeResponseBody for &'static str {}
impl ResponseBody for &'static str {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for String {}
impl ResponseBody for String {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for Box<str> {}
impl ResponseBody for Box<str> {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for Cow<'static, str> {}
impl ResponseBody for Cow<'static, str> {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for &'static [u8] {}
impl ResponseBody for &'static [u8] {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<const N: usize> ShouldBeResponseBody for &'static [u8; N] {}
impl<const N: usize> ResponseBody for &'static [u8; N] {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<const N: usize> ShouldBeResponseBody for [u8; N] {}
impl<const N: usize> ResponseBody for [u8; N] {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Vec<u8> {}
impl ResponseBody for Vec<u8> {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Box<[u8]> {}
impl ResponseBody for Box<[u8]> {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Bytes {}
impl ResponseBody for Bytes {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for BytesMut {}
impl ResponseBody for BytesMut {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Cow<'static, [u8]> {}
impl ResponseBody for Cow<'static, [u8]> {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<T> ShouldBeResponseBody for Json<T> {}
impl<T: Serialize + JsonSchema> ResponseBody for Json<T> {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(
            StatusCode::OK,
            Some((mime::APPLICATION_JSON, Some(_gen.generate::<T>()))),
        )]
    }
}

impl ShouldBeResponseBody for () {}
impl ResponseBody for () {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
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
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        let mut bodies = T::body(&mut *_gen);
        bodies.extend(E::body(&mut *_gen));
        bodies
    }
}

impl ShouldBeResponseBody for Redirect {}
impl ResponseBody for Redirect {
    fn header() -> Vec<HeaderName> {
        vec![header::LOCATION]
    }

    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![
            (StatusCode::SEE_OTHER, None),
            (StatusCode::TEMPORARY_REDIRECT, None),
            (StatusCode::PERMANENT_REDIRECT, None),
        ]
    }
    // fn responses(gen: &mut SchemaGenerator) -> Responses {
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
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<T> ShouldBeResponseBody for Html<T> {}
impl<T> ResponseBody for Html<T> {
    fn body(_gen: &mut SchemaGenerator) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_HTML_UTF_8, None)))]
    }
}
