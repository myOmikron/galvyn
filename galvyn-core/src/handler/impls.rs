use std::any::type_name;
use std::borrow::Cow;
use std::sync::LazyLock;

use axum::Form;
use axum::Json;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::RawForm;
use axum::http::HeaderName;
use axum::http::Method;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::Html;
use axum::response::Redirect;
use bytes::Buf;
use bytes::BytesMut;
use bytes::buf::Chain;
use mime::Mime;
use regex::Regex;
use schemars::JsonSchema;
use schemars::schema::InstanceType;
use schemars::schema::Schema;
use schemars::schema::SingleOrVec;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;

use super::request_body::RequestBody;
use super::request_body::ShouldBeRequestBody;
use super::request_part::RequestPart;
use super::request_part::ShouldBeRequestPart;
use crate::handler::context::EndpointContext;
use crate::handler::response_body::ResponseBody;
use crate::handler::response_body::ShouldBeResponseBody;

impl ShouldBeRequestBody for String {}
impl RequestBody for String {
    fn body(_ctx: &mut EndpointContext) -> (Mime, Option<Schema>) {
        (mime::TEXT_PLAIN_UTF_8, None)
    }
}

impl ShouldBeRequestBody for Bytes {}
impl RequestBody for Bytes {
    fn body(_ctx: &mut EndpointContext) -> (Mime, Option<Schema>) {
        (mime::APPLICATION_OCTET_STREAM, None)
    }
}

impl<T> ShouldBeRequestBody for Json<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestBody for Json<T> {
    fn body(ctx: &mut EndpointContext) -> (Mime, Option<Schema>) {
        (mime::APPLICATION_JSON, Some(ctx.generator.generate::<T>()))
    }
}

impl<T> ShouldBeRequestBody for Form<T> {}

impl<T: DeserializeOwned + JsonSchema> RequestBody for Form<T> {
    fn query_parameters(ctx: &mut EndpointContext) -> Vec<(String, Option<Schema>)> {
        if ctx.method == Method::GET {
            let Some(obj) = ctx.generator.generate_object::<T>() else {
                warn!("Unsupported handler argument: {}", type_name::<Self>());
                return Vec::new();
            };

            obj.properties
                .into_iter()
                .map(|(name, schema)| (name, Some(schema)))
                .collect()
        } else {
            vec![]
        }
    }

    fn body(ctx: &mut EndpointContext) -> (Mime, Option<Schema>) {
        if ctx.method == Method::GET {
            RawForm::body(ctx)
        } else {
            (
                mime::APPLICATION_WWW_FORM_URLENCODED,
                Some(ctx.generator.generate::<T>()),
            )
        }
    }
}

impl ShouldBeRequestBody for RawForm {}
impl RequestBody for RawForm {
    fn body(ctx: &mut EndpointContext) -> (Mime, Option<Schema>) {
        if ctx.method == Method::GET {
            // This is a dirty hack.
            //
            // The "correct" implementation would be returning `None`.
            // However, a type implementing `RequestBody` should actually process the body.
            // The only exceptions I know of are `Form` and `RawForm` and I don't want to
            // add complexity for every other type just to support them.
            (
                "application/x-empty"
                    .parse()
                    .expect("This should be a valid mime type"),
                None,
            )
        } else {
            (mime::APPLICATION_WWW_FORM_URLENCODED, None)
        }
    }
}

static PATH_PARAM_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{[^}]*}").unwrap());
impl<T> ShouldBeRequestPart for Path<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestPart for Path<T> {
    fn path_parameters(ctx: &mut EndpointContext) -> Vec<(String, Option<Schema>)> {
        let Schema::Object(schema) = ctx.generator.generate_refless::<T>() else {
            warn!(type = type_name::<T>(), "Failed to generate schema, this should never happen!");
            return Vec::new();
        };

        let mut unhandled_params: Vec<_> = PATH_PARAM_REGEX
            .find_iter(ctx.path)
            .map(|needle| &ctx.path[(needle.start() + 1)..(needle.end() - 1)])
            .collect();
        let mut handled_params = Vec::new();

        match &schema.instance_type {
            Some(SingleOrVec::Single(boxed)) if **boxed == InstanceType::Object => {
                let object = schema.object.unwrap_or_else(|| {
                    warn!("Unsupported handler argument: {}", type_name::<Self>());
                    Default::default()
                });

                handled_params.extend(
                    object
                        .properties
                        .into_iter()
                        .map(|(name, schema)| (name, Some(schema))),
                );

                for (key, _) in &handled_params {
                    unhandled_params.retain_mut(|path_key| path_key != key);
                }
            }
            Some(SingleOrVec::Single(boxed)) if **boxed == InstanceType::Array => {
                let array = schema.array.unwrap_or_else(|| {
                    warn!("Unsupported handler argument: {}", type_name::<Self>());
                    Default::default()
                });
                let items = array.items.unwrap_or_else(|| {
                    warn!("Unsupported handler argument: {}", type_name::<Self>());
                    SingleOrVec::Vec(Vec::new())
                });

                match items {
                    SingleOrVec::Single(item) => {
                        handled_params.extend(
                            unhandled_params
                                .drain(..)
                                .map(|key| (key.to_string(), Some(item.as_ref().clone()))),
                        );
                    }
                    SingleOrVec::Vec(items) => {
                        if items.len() > unhandled_params.len() {
                            warn!(
                                schema = type_name::<Self>(),
                                schema.len = items.len(),
                                path = ctx.path,
                                path.len = unhandled_params.len(),
                                "Path parameters don't cover entire schema",
                            );
                        }
                        handled_params.extend(
                            unhandled_params
                                .drain(..items.len())
                                .zip(items)
                                .map(|(key, schema)| (key.to_string(), Some(schema))),
                        );
                    }
                }
            }
            Some(SingleOrVec::Single(_)) => {
                if unhandled_params.is_empty() {
                    warn!(
                        schema = type_name::<Self>(),
                        path = ctx.path,
                        "Missing path parameter",
                    );
                } else {
                    handled_params.push((
                        unhandled_params.remove(0).to_string(),
                        Some(Schema::Object(schema)),
                    ));
                }
            }
            _ => {
                warn!("Unsupported handler argument: {}", type_name::<Self>());
            }
        }

        if unhandled_params.is_empty() {
            handled_params
        } else {
            warn!(
                schema = type_name::<Self>(),
                schema.len = handled_params.len(),
                path = ctx.path,
                path.len = handled_params.len() + unhandled_params.len(),
                "Schema does not cover all path parameters",
            );
            handled_params.extend(unhandled_params.iter().map(|key| (key.to_string(), None)));
            handled_params
        }
    }
}

impl<T> ShouldBeRequestPart for Query<T> {}
impl<T: DeserializeOwned + JsonSchema> RequestPart for Query<T> {
    fn query_parameters(ctx: &mut EndpointContext) -> Vec<(String, Option<Schema>)> {
        let Some(obj) = ctx.generator.generate_object::<T>() else {
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
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for String {}
impl ResponseBody for String {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for Box<str> {}
impl ResponseBody for Box<str> {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for Cow<'static, str> {}
impl ResponseBody for Cow<'static, str> {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_PLAIN_UTF_8, None)))]
    }
}

impl ShouldBeResponseBody for &'static [u8] {}
impl ResponseBody for &'static [u8] {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<const N: usize> ShouldBeResponseBody for &'static [u8; N] {}
impl<const N: usize> ResponseBody for &'static [u8; N] {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<const N: usize> ShouldBeResponseBody for [u8; N] {}
impl<const N: usize> ResponseBody for [u8; N] {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Vec<u8> {}
impl ResponseBody for Vec<u8> {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Box<[u8]> {}
impl ResponseBody for Box<[u8]> {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Bytes {}
impl ResponseBody for Bytes {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for BytesMut {}
impl ResponseBody for BytesMut {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl ShouldBeResponseBody for Cow<'static, [u8]> {}
impl ResponseBody for Cow<'static, [u8]> {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<T> ShouldBeResponseBody for Json<T> {}
impl<T: Serialize + JsonSchema> ResponseBody for Json<T> {
    fn body(ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(
            StatusCode::OK,
            Some((mime::APPLICATION_JSON, Some(ctx.generator.generate::<T>()))),
        )]
    }
}

impl ShouldBeResponseBody for () {}
impl ResponseBody for () {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
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
    fn body(generator: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
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

    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![
            (StatusCode::SEE_OTHER, None),
            (StatusCode::TEMPORARY_REDIRECT, None),
            (StatusCode::PERMANENT_REDIRECT, None),
        ]
    }
    // fn responses(generator: &mut HandlerContext) -> Responses {
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
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::APPLICATION_OCTET_STREAM, None)))]
    }
}

impl<T> ShouldBeResponseBody for Html<T> {}
impl<T> ResponseBody for Html<T> {
    fn body(_ctx: &mut EndpointContext) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
        vec![(StatusCode::OK, Some((mime::TEXT_HTML_UTF_8, None)))]
    }
}
