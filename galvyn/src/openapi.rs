use crate::get_routes;
use axum::http::Method;
use galvyn_core::re_exports::schemars;
use galvyn_core::schema_generator::SchemaGenerator;
use openapiv3::{
    Components, Info, MediaType, Parameter, ParameterData, ParameterSchemaOrContent, PathItem,
    Paths, ReferenceOr, RequestBody, Response, Schema, SchemaKind, StatusCode,
};
use std::collections::BTreeMap;
use std::mem;
use std::sync::OnceLock;
use tracing::{debug, warn};

pub use openapiv3::OpenAPI;
use serde_json::Error;

pub fn get_openapi() -> &'static OpenAPI {
    static OPENAPI: OnceLock<OpenAPI> = OnceLock::new();
    OPENAPI.get_or_init(generate_openapi)
}

fn generate_openapi() -> OpenAPI {
    let mut schemas: BTreeMap<String, schemars::schema::Schema> = BTreeMap::new();
    let mut paths = Paths::default();

    for route in get_routes() {
        let ReferenceOr::Item(path) = paths
            .paths
            .entry(route.path.to_string())
            .or_insert_with(|| ReferenceOr::Item(PathItem::default()))
        else {
            unreachable!("We only ever insert ReferenceOr::Item. See above")
        };

        let operation = match route.method {
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

        operation.summary = route.doc.get(0).map(|line| line.trim().to_string());
        if let Some((head, rest)) = route.doc.split_first() {
            let description = operation.description.insert(head.trim().to_string());
            for line in rest {
                description.push('\n');
                description.push_str(line.trim());
            }
        }
        operation.operation_id = Some(route.ident.to_string());
        operation.deprecated = route.deprecated;
        operation.tags = route.tags.iter().copied().map(String::from).collect();

        SchemaGenerator::employ(&mut schemas, |gen| {
            if let Some(response_body) = route.response_body.as_ref() {
                for (status_code, body) in (response_body.body)(gen) {
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
                                route.ident,
                                reason = "Schema is not proper openapiv3",
                                "Malformed response body schema"
                            );
                            debug!(
                                route.ident,
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
            if let Some(request_body) = route.request_body.as_ref() {
                let (mime, schema) = (request_body.body)(gen);
                operation.request_body = Some(ReferenceOr::Item(RequestBody {
                    content: FromIterator::from_iter([(
                        mime.to_string(),
                        MediaType {
                            schema: schema.as_ref().map(convert_schema).and_then(|result| {
                                result
                                    .inspect_err(|error| {
                                        warn!(
                                            route.ident,
                                            reason = "Schema is not proper openapiv3",
                                            "Malformed request body schema"
                                        );
                                        debug!(
                                            route.ident,
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
            for part in &route.request_parts {
                for (name, schema) in (part.path_parameters)(&mut *gen) {
                    operation
                        .parameters
                        .push(ReferenceOr::Item(Parameter::Path {
                            parameter_data: ParameterData {
                                required: true,
                                ..convert_parameter(name, schema)
                            },
                            style: Default::default(),
                        }));
                }
                for (name, schema) in (part.query_parameters)(&mut *gen) {
                    operation
                        .parameters
                        .push(ReferenceOr::Item(Parameter::Query {
                            parameter_data: convert_parameter(name, schema),
                            allow_reserved: Default::default(),
                            style: Default::default(),
                            allow_empty_value: Default::default(),
                        }));
                }
            }
            for part in &route.response_parts {
                // TODO
            }
        });
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
                .into_iter()
                .filter_map(|(key, schema)| match convert_schema(&schema) {
                    Ok(schema) => Some((key, schema)),
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

fn convert_parameter(name: String, schema: Option<schemars::schema::Schema>) -> ParameterData {
    ParameterData {
        name,
        description: None,
        required: false,
        deprecated: None,
        format: ParameterSchemaOrContent::Schema(
            schema
                .and_then(|schema| match convert_schema(&schema) {
                    Ok(schema) => Some(schema),
                    Err(_) => None,
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
