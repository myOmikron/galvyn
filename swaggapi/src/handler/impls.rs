use super::request_body::{RequestBody, ShouldBeRequestBody};
use super::request_part::{RequestPart, ShouldBeRequestPart};
use crate::handler::response_body::{ResponseBody, ShouldBeResponseBody};
use crate::schema_generator::SchemaGenerator;
use crate::utils::SchemalessJson;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::RawForm;
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

impl<T> ShouldBeRequestBody for SchemalessJson<T> {}
impl<T: DeserializeOwned> RequestBody for SchemalessJson<T> {
    fn body(_gen: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
        (mime::APPLICATION_JSON, None)
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
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_text()
    // }
}

impl ShouldBeResponseBody for String {}
impl ResponseBody for String {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_text()
    // }
}

impl ShouldBeResponseBody for Box<str> {}
impl ResponseBody for Box<str> {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_text()
    // }
}

impl ShouldBeResponseBody for Cow<'static, str> {}
impl ResponseBody for Cow<'static, str> {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_text()
    // }
}

impl ShouldBeResponseBody for &'static [u8] {}
impl ResponseBody for &'static [u8] {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl<const N: usize> ShouldBeResponseBody for &'static [u8; N] {}
impl<const N: usize> ResponseBody for &'static [u8; N] {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl<const N: usize> ShouldBeResponseBody for [u8; N] {}
impl<const N: usize> ResponseBody for [u8; N] {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl ShouldBeResponseBody for Vec<u8> {}
impl ResponseBody for Vec<u8> {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl ShouldBeResponseBody for Box<[u8]> {}
impl ResponseBody for Box<[u8]> {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl ShouldBeResponseBody for Bytes {}
impl ResponseBody for Bytes {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl ShouldBeResponseBody for BytesMut {}
impl ResponseBody for BytesMut {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl ShouldBeResponseBody for Cow<'static, [u8]> {}
impl ResponseBody for Cow<'static, [u8]> {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl<T> ShouldBeResponseBody for Json<T> {}
impl<T: Serialize + JsonSchema> ResponseBody for Json<T> {
    // fn responses(gen: &mut SchemaGenerator) -> Responses {
    //     ok_json::<T>(gen)
    // }
}

impl<T> ShouldBeResponseBody for SchemalessJson<T> {}
impl<T: Serialize> ResponseBody for SchemalessJson<T> {
    // fn responses(_: &mut SchemaGenerator) -> Responses {
    //     simple_responses([
    //         SimpleResponse {
    //             status_code: StatusCode::Code(200),
    //             mime_type: mime::APPLICATION_JSON,
    //             description: "Some json data".to_string(),
    //             media_type: Some(MediaType {
    //                 schema: Some(ReferenceOr::Item(Schema {
    //                     schema_data: Default::default(),
    //                     schema_kind: SchemaKind::Any(Default::default()),
    //                 })),
    //                 ..Default::default()
    //             }),
    //         },
    //         // TODO add error
    //     ])
    // }
}

impl ShouldBeResponseBody for () {}
impl ResponseBody for () {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_empty()
    // }
}

impl<T, E> ShouldBeResponseBody for Result<T, E> {}
impl<T, E> ResponseBody for Result<T, E>
where
    T: ResponseBody,
    E: ResponseBody,
{
    // fn responses(gen: &mut SchemaGenerator) -> Responses {
    //     let mut res = E::responses(gen);
    //     let ok_res = T::responses(gen);
    //
    //     // As we want to preserve in almost any cases the Ok branch of the result, we're extending
    //     // the IndexMaps of the error-branch with those of the ok-branch
    //     res.responses.extend(ok_res.responses);
    //     res.extensions.extend(ok_res.extensions);
    //     if ok_res.default.is_some() {
    //         res.default = ok_res.default;
    //     }
    //
    //     res
    // }
}

impl ShouldBeResponseBody for Redirect {}
impl ResponseBody for Redirect {
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
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_binary()
    // }
}

impl<T> ShouldBeResponseBody for Html<T> {}
impl<T> ResponseBody for Html<T> {
    // fn responses(_gen: &mut SchemaGenerator) -> Responses {
    //     ok_html()
    // }
}
