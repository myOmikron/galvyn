use std::mem;

use axum::http::Method;
use galvyn_core::handler::context::EndpointContext;
use galvyn_core::re_exports::schemars;
use galvyn_core::router::GalvynRoute;
use galvyn_core::schema_generator::SchemaGenerator;
use openapiv3::Components;
use openapiv3::Header;
use openapiv3::Info;
use openapiv3::MediaType;
pub use openapiv3::OpenAPI;
use openapiv3::Parameter;
use openapiv3::ParameterData;
use openapiv3::ParameterSchemaOrContent;
use openapiv3::PathItem;
use openapiv3::Paths;
use openapiv3::ReferenceOr;
use openapiv3::RequestBody;
use openapiv3::Response;
use openapiv3::Schema;
use openapiv3::SchemaKind;
use openapiv3::StatusCode;
use openapiv3::Type;
use tracing::debug;
use tracing::warn;

use crate::openapi::OpenapiBuilder;
use crate::openapi::OpenapiMetadata;
use crate::Galvyn;

pub fn generate_openapi(builder: &OpenapiBuilder) -> OpenAPI {
    let mut schemas = SchemaGenerator::new();
    let mut paths = Paths::default();
    let default_metadata = OpenapiMetadata::default();

    for route in Galvyn::global().get_routes() {
        let openapi_ext = route.extensions.get().unwrap_or(&default_metadata);
        if let Some(pages) = &builder.private.pages {
            if !openapi_ext.pages.iter().any(|page| pages.contains(page)) {
                continue;
            }
        }

        let ReferenceOr::Item(path) = paths
            .paths
            .entry(route.path.to_string())
            .or_insert_with(|| ReferenceOr::Item(PathItem::default()))
        else {
            unreachable!("We only ever insert ReferenceOr::Item. See above")
        };

        let operation = match route.handler.method {
            Method::GET => &mut path.get,
            Method::POST => &mut path.post,
            Method::PUT => &mut path.put,
            Method::DELETE => &mut path.delete,
            Method::HEAD => &mut path.head,
            Method::OPTIONS => &mut path.options,
            Method::PATCH => &mut path.patch,
            Method::TRACE => &mut path.trace,
            _ => unimplemented!("We don't support custom methods"),
        };
        let operation = operation.get_or_insert_default();

        operation.summary = route
            .handler
            .doc
            .first()
            .map(|line| line.trim().to_string());
        if let Some((head, rest)) = route.handler.doc.split_first() {
            let description = operation.description.insert(head.trim().to_string());
            for line in rest {
                description.push('\n');
                description.push_str(line.trim());
            }
        }
        operation.operation_id = Some(route.handler.ident.to_string());
        operation.deprecated = route.handler.deprecated;
        if !builder.omit_tags {
            operation.tags = openapi_ext.tags.iter().copied().map(String::from).collect();
        }

        let mut ctx = EndpointContext::_new(&mut schemas, &route.handler.method, &route.path);
        if let Some(response_body) = route.handler.response_body.as_ref() {
            for (status_code, body) in (response_body.body)(&mut ctx) {
                // Insert status code
                let ReferenceOr::Item(response) = operation
                    .responses
                    .responses
                    .entry(StatusCode::Code(status_code.as_u16()))
                    .or_insert_with(|| ReferenceOr::Item(Response::default()))
                else {
                    unreachable!("We only ever insert ReferenceOr::Item. See above")
                };

                // Insert mime type
                let Some((mime, schema)) = body else {
                    continue;
                };
                let media_type = response.content.entry(mime.to_string()).or_default();

                // Insert schema
                let Some(schema) = schema else {
                    continue;
                };
                let schema = match convert_schema(&schema) {
                    Ok(schema) => schema,
                    Err(error) => {
                        warn!(
                            route.handler.ident,
                            route.path,
                            reason = "Schema is not proper openapiv3",
                            "Malformed response body schema"
                        );
                        debug!(
                            route.handler.ident,
                            route.path,
                            reason = "Schema is not proper openapiv3",
                            error.display = %error,
                            error.debug = ?error,
                            "Malformed response body schema"
                        );
                        continue;
                    }
                };
                match &mut media_type.schema {
                    // We add the 1st schema
                    None => media_type.schema = Some(schema),
                    // We add the 3rd or further schema
                    Some(ReferenceOr::Item(Schema {
                        schema_data: _,
                        schema_kind: SchemaKind::OneOf { one_of },
                    })) => {
                        one_of.push(schema);
                    }
                    // We add the 2nd schema
                    Some(schema_slot) => {
                        let other_schema = mem::replace(
                            schema_slot,
                            ReferenceOr::Reference {
                                reference: String::new(),
                            },
                        );
                        *schema_slot = ReferenceOr::Item(Schema {
                            schema_data: Default::default(),
                            schema_kind: SchemaKind::OneOf {
                                one_of: vec![other_schema, schema],
                            },
                        });
                    }
                };
            }
        }
        if let Some(request_body) = route.handler.request_body.as_ref() {
            let (mime, schema) = (request_body.body)(&mut ctx);
            operation.request_body = Some(ReferenceOr::Item(RequestBody {
                content: FromIterator::from_iter([(
                    mime.to_string(),
                    MediaType {
                        schema: schema.as_ref().map(convert_schema).and_then(|result| {
                            result
                                .inspect_err(|error| {
                                    warn!(
                                        route.handler.ident,
                                        route.path,
                                        reason = "Schema is not proper openapiv3",
                                        "Malformed request body schema"
                                    );
                                    debug!(
                                        route.handler.ident,
                                        route.path,
                                        reason = "Schema is not proper openapiv3",
                                        error.display = %error,
                                        error.debug = ?error,
                                        "Malformed request body schema"
                                    );
                                })
                                .ok()
                        }),
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            }));
        }
        for part in &route.handler.request_parts {
            for (name, schema) in (part.path_parameters)(&mut ctx) {
                operation
                    .parameters
                    .push(ReferenceOr::Item(Parameter::Path {
                        parameter_data: ParameterData {
                            required: true,
                            ..convert_parameter(name, schema, route)
                        },
                        style: Default::default(),
                    }));
            }
            for (name, schema) in (part.query_parameters)(&mut ctx) {
                operation
                    .parameters
                    .push(ReferenceOr::Item(Parameter::Query {
                        parameter_data: convert_parameter(name, schema, route),
                        allow_reserved: Default::default(),
                        style: Default::default(),
                        allow_empty_value: Default::default(),
                    }));
            }
        }
        for part in &route.handler.response_parts {
            for response_or_ref in operation
                .responses
                .default
                .iter_mut()
                .chain(operation.responses.responses.values_mut())
            {
                let response_headers = match response_or_ref {
                    ReferenceOr::Item(response) => &mut response.headers,
                    ReferenceOr::Reference { .. } => {
                        // Our code should use references for responses
                        warn!("This is a bug in galvyn");
                        continue;
                    }
                };
                for header in (part.header)() {
                    response_headers
                        .entry(header.to_string())
                        .or_insert(ReferenceOr::Item(Header {
                            format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                                schema_data: Default::default(),
                                schema_kind: SchemaKind::Type(Type::String(Default::default())),
                            })),
                            description: Default::default(),
                            style: Default::default(),
                            required: Default::default(),
                            deprecated: Default::default(),
                            example: Default::default(),
                            examples: Default::default(),
                            extensions: Default::default(),
                        }));
                }
            }
        }
    }

    OpenAPI {
        openapi: "3.0.0".to_string(),
        info: Info {
            title: "Unnamed Galvyn API".to_string(),
            description: None,
            terms_of_service: None,
            contact: None,
            license: None,
            version: "v0.0.0".to_string(),
            extensions: Default::default(),
        },
        servers: vec![],
        paths,
        components: Some(Components {
            schemas: schemas
                .into_definitions()
                .iter()
                .filter_map(|(key, schema)| match convert_schema(schema) {
                    Ok(schema) => Some((key.clone(), schema)),
                    Err(error) => {
                        warn!(
                            schema = key,
                            reason = "Schema is not proper openapiv3",
                            "Malformed schema"
                        );
                        debug!(
                            schema = key,
                            reason = "Schema is not proper openapiv3",
                            error.display = %error,
                            error.debug = ?error,
                            "Malformed schema"
                        );
                        None
                    }
                })
                .collect(),
            ..Default::default()
        }),
        security: None,
        tags: vec![],
        external_docs: None,
        extensions: Default::default(),
    }
}

fn convert_parameter(
    name: String,
    schema: Option<schemars::schema::Schema>,
    route: &GalvynRoute,
) -> ParameterData {
    ParameterData {
        name,
        description: None,
        required: false,
        deprecated: None,
        format: ParameterSchemaOrContent::Schema(
            schema
                .and_then(|schema| match convert_schema(&schema) {
                    Ok(schema) => Some(schema),
                    Err(error) => {
                        warn!(
                            route.handler.ident,
                            route.path,
                            reason = "Schema is not proper openapiv3",
                            "Malformed request parameter schema"
                        );
                        debug!(
                            route.handler.ident,
                            route.path,
                            reason = "Schema is not proper openapiv3",
                            error.display = %error,
                            error.debug = ?error,
                            "Malformed request parameter schema"
                        );
                        None
                    }
                })
                .unwrap_or_else(|| {
                    ReferenceOr::Item(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Any(Default::default()),
                    })
                }),
        ),
        example: None,
        examples: Default::default(),
        explode: None,
        extensions: Default::default(),
    }
}

fn convert_schema(
    schema: &schemars::schema::Schema,
) -> Result<ReferenceOr<Schema>, serde_json::Error> {
    serde_json::to_string(schema).and_then(|string| serde_json::from_str(&string))
}

#[cfg(test)]
mod tests {
    use galvyn_core::stuff::schema::FormErrorResponse;
    use galvyn_core::stuff::schema::FormResult;
    use galvyn_core::stuff::schema::Optional;
    use galvyn_core::stuff::schema::Page;
    use galvyn_core::stuff::schema::SchemaDateTime;
    use galvyn_core::stuff::schema::SingleLink;
    use galvyn_core::stuff::schema::SingleUuid;
    use serde_json::json;
    use serde_json::Value;

    use super::*;

    fn convert(schema: &schemars::schema::Schema) -> Result<Value, serde_json::Error> {
        convert_schema(schema).map(|schema| serde_json::to_value(schema).unwrap())
    }

    #[test]
    fn convert_ref() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            convert(&generator.generate::<SingleUuid>()).unwrap(),
            json!({"$ref": "#/components/schemas/SingleUuid"})
        );
    }

    #[test]
    fn convert_struct() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            convert(&generator.generate_refless::<Page<SingleUuid>>()).unwrap(),
            json!({
                "description": "A page of items",
                "type": "object",
                "required": ["items", "limit", "offset", "total"],
                "properties": {
                    "items": {
                        "description": "The page's items",
                        "type": "array",
                        "items": {"$ref": "#/components/schemas/SingleUuid"}
                    },
                    "limit": {
                        "description": "The limit this page was requested with",
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0.0
                    },
                    "offset": {
                        "description": "The offset this page was requested with",
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0.0
                    },
                    "total": {
                        "description": "The total number of items this page is a subset of",
                        "type": "integer",
                        "format": "int64"
                    }
                }
            })
        );
    }

    #[test]
    fn convert_nullable() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            convert(&generator.generate::<Option<String>>()).unwrap(),
            json!({"type": "string", "nullable": true})
        );
        assert_eq!(
            convert(&generator.generate::<Option<SingleUuid>>()).unwrap(),
            json!({"allOf": [{"$ref": "#/components/schemas/SingleUuid"}], "nullable": true})
        );
        assert_eq!(
            convert(&generator.generate::<()>()).unwrap(),
            json!({"nullable": true})
        );
        assert_eq!(
            convert(&generator.generate_refless::<Optional<()>>()).unwrap()["properties"],
            json!({"optional": {"nullable": true}})
        );
    }

    #[test]
    fn convert_formatted_string() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            convert(&generator.generate_refless::<SchemaDateTime>()).unwrap(),
            json!({"type": "string", "format": "date-time", "example": "1970-01-01T00:00:00.0Z"})
        );
    }

    #[test]
    fn convert_tagged_enum() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            convert(&generator.generate_refless::<FormResult<SingleUuid, SingleLink>>()).unwrap(),
            json!({
                "description": "A `Result` with a custom serialization",
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["result", "value"],
                        "properties": {
                            "result": {"type": "string", "enum": ["Ok"]},
                            "value": {"$ref": "#/components/schemas/SingleUuid"}
                        }
                    },
                    {
                        "type": "object",
                        "required": ["error", "result"],
                        "properties": {
                            "error": {"$ref": "#/components/schemas/SingleLink"},
                            "result": {"type": "string", "enum": ["Err"]}
                        }
                    }
                ]
            })
        );
    }

    /// Openapi 3.0 doesn't support an array of `items`.
    ///
    /// Tuples are currently rejected and dropped by `generate_openapi`.
    #[test]
    fn convert_tuple_fails() {
        let mut generator = SchemaGenerator::new();
        assert!(convert(&generator.generate::<(String, u32)>()).is_err());
    }

    /// Bool schemas (as used by `galvyn-contrib-auth`) are not valid openapiv3 schemas.
    ///
    /// They are currently rejected and dropped by `generate_openapi`.
    #[test]
    fn convert_bool_schema_fails() {
        assert!(convert(&schemars::schema::Schema::Bool(true)).is_err());
        assert!(convert(&schemars::schema::Schema::Bool(false)).is_err());
    }

    #[test]
    fn convert_definitions() {
        let mut generator = SchemaGenerator::new();
        generator.generate::<FormErrorResponse<SingleLink>>();
        generator.generate::<FormResult<SingleUuid, SingleLink>>();
        generator.generate::<Optional<SchemaDateTime>>();
        generator.generate::<Page<SingleUuid>>();

        let definitions = generator.into_definitions();
        let mut keys: Vec<_> = definitions.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "ErrorConstant",
                "FormErrorResponse_for_SingleLink",
                "FormResult_for_SingleUuid_and_SingleLink",
                "Optional_for_SchemaDateTime",
                "Page_for_SingleUuid",
                "SchemaDateTime",
                "SingleLink",
                "SingleUuid",
            ]
        );
        for (key, schema) in &definitions {
            if let Err(error) = convert(schema) {
                panic!("Failed to convert {key}: {error}");
            }
        }
    }
}
