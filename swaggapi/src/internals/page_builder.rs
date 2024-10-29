use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use openapiv3::Components;
use openapiv3::Contact;
use openapiv3::Info;
use openapiv3::License;
use openapiv3::OpenAPI;
use openapiv3::Operation;
use openapiv3::PathItem;
use openapiv3::Paths;
use openapiv3::ReferenceOr;
use regex::Regex;
use schemars::schema::Schema;
use schemars::JsonSchema;

use crate::internals::HttpMethod;
use crate::internals::SchemaGenerator;
use crate::internals::{convert_schema, ContextHandler};
use crate::page::SwaggapiPageBuilder;

/// This trait associates one static instance of a [`SwaggapiPageBuilder`] to its implementor.
///
/// It is the implementation detail of [`SwaggapiPage`](trait@crate::SwaggapiPage)
/// and should be implemented through [`#[derive(SwaggapiPage)]`](macro@crate::SwaggapiPage).
pub trait AccessSwaggapiPageBuilder {
    /// Get the [`SwaggapiPageBuilder`]
    ///
    /// This method will always (using [`#[derive(SwaggapiPage)]`](macro@crate::SwaggapiPage)) be implemented as:
    /// ```rust
    /// # use swaggapi::SwaggapiPageBuilder;
    /// # use swaggapi::internals::AccessSwaggapiPageBuilder;
    /// # struct Test;
    /// # impl AccessSwaggapiPageBuilder for Test {
    /// fn get_builder(&self) -> &'static SwaggapiPageBuilder {
    ///     static BUILDER: SwaggapiPageBuilder = SwaggapiPageBuilder::new();
    ///     &BUILDER
    /// }
    /// # }
    /// ```
    fn get_builder(&self) -> &'static SwaggapiPageBuilder;
}
impl AccessSwaggapiPageBuilder for &'static SwaggapiPageBuilder {
    fn get_builder(&self) -> &'static SwaggapiPageBuilder {
        *self
    }
}

/// The parts of a [`SwaggapiPageBuilder`] which are considered [internal](crate::internals)
///
/// This struct implements the actual construction of an [`OpenAPI`] document
/// combining the handlers added trough [`SwaggapiPageBuilderImpl::add_handler`]
/// and the metadata stored in [`SwaggapiPageBuilder`].
#[derive(Default)]
pub struct SwaggapiPageBuilderImpl {
    paths: Paths,

    schemas: BTreeMap<String, Schema>,

    /// Cache for the result of [`SwaggapiPageBuilder::build`]
    last_build: Option<Arc<OpenAPI>>,
}

impl SwaggapiPageBuilderImpl {
    /// Add a [`JsonSchema`] to this api page
    ///
    /// Might be useful to explicitly include a schema which is used by any handler.
    pub fn add_schema<T: JsonSchema>(builder: &SwaggapiPageBuilder) {
        let mut guard = builder.state.lock().unwrap();
        let state = guard.get_or_insert_with(Default::default);
        state.last_build = None;

        SchemaGenerator::employ(&mut state.schemas, |gen| gen.generate::<T>());
    }

    /// Add a handler to this api page
    pub fn add_handler(builder: &SwaggapiPageBuilder, handler: &ContextHandler) {
        let mut guard = builder.state.lock().unwrap();
        let state = guard.get_or_insert_with(Default::default);
        state.last_build = None;

        let (parameters, mut request_body, responses) =
            SchemaGenerator::employ(&mut state.schemas, |gen| {
                let mut parameters = Vec::new();
                let mut request_body = Vec::new();
                for arg in handler.handler_arguments {
                    if let Some(arg) = arg.as_ref() {
                        static PATH_PARAM_REGEX: OnceLock<Regex> = OnceLock::new();
                        let path_param_regex =
                            PATH_PARAM_REGEX.get_or_init(|| Regex::new(r"\{[^}]*}").unwrap());
                        let path_params = path_param_regex
                            .find_iter(&handler.path)
                            .map(|needle| &handler.path[(needle.start() + 1)..(needle.end() - 1)])
                            .collect::<Vec<_>>();

                        parameters.extend(
                            (arg.parameters)(&mut *gen, &path_params)
                                .into_iter()
                                .map(ReferenceOr::Item),
                        );
                        request_body.extend((arg.request_body)(&mut *gen));
                    }
                }
                let responses = (handler.responses)(&mut *gen);
                (parameters, request_body, responses)
            });

        let summary = handler.doc.get(0).map(|line| line.trim().to_string());
        let description = summary.clone().map(|summary| {
            handler
                .doc
                .get(1..)
                .unwrap_or(&[])
                .iter()
                .fold(summary, |text, line| format!("{text}\n{}", line.trim()))
        });

        let operation = Operation {
            summary,
            description,
            operation_id: Some(handler.ident.to_string()),
            parameters,
            request_body: request_body.pop().map(ReferenceOr::Item),
            responses,
            deprecated: handler.deprecated,
            security: None, // TODO
            tags: handler.tags.iter().map(String::from).collect(),
            // Not supported:
            external_docs: Default::default(),
            servers: Default::default(),
            extensions: Default::default(),
            callbacks: Default::default(),
        };

        let ReferenceOr::Item(path) = state
            .paths
            .paths
            .entry(handler.path.to_string())
            .or_insert_with(|| ReferenceOr::Item(PathItem::default()))
        else {
            unreachable!("We only ever insert ReferenceOr::Item. See above")
        };
        let operation_mut = match handler.method {
            HttpMethod::Get => &mut path.get,
            HttpMethod::Post => &mut path.post,
            HttpMethod::Put => &mut path.put,
            HttpMethod::Delete => &mut path.delete,
            HttpMethod::Head => &mut path.head,
            HttpMethod::Options => &mut path.options,
            HttpMethod::Patch => &mut path.patch,
            HttpMethod::Trace => &mut path.trace,
        };
        *operation_mut = Some(operation);
    }

    /// Returns the [`OpenAPI`] file
    ///
    /// The build operation is cached (hence the `Arc`) so feel free to call this eagerly.
    pub fn build(builder: &SwaggapiPageBuilder) -> Arc<OpenAPI> {
        let SwaggapiPageBuilder {
            title,
            description,
            terms_of_service,
            contact_name,
            contact_url,
            contact_email,
            license_name,
            license_url,
            version,
            filename: _,
            state,
        } = builder;
        let mut guard = state.lock().unwrap();
        let state = guard.get_or_insert_with(Default::default);

        if let Some(open_api) = state.last_build.clone() {
            return open_api;
        }

        let open_api = Arc::new(OpenAPI {
            openapi: "3.0.0".to_string(),
            info: Info {
                title: title.unwrap_or("Unnamed API").to_string(),
                description: description.map(str::to_string),
                terms_of_service: terms_of_service.map(str::to_string),
                contact: (contact_name.is_some()
                    || contact_url.is_some()
                    || contact_email.is_some())
                .then(|| Contact {
                    name: contact_name.map(str::to_string),
                    url: contact_url.map(str::to_string),
                    email: contact_email.map(str::to_string),
                    extensions: Default::default(),
                }),
                license: (license_name.is_some() || license_url.is_some()).then(|| License {
                    name: builder
                        .license_name
                        .unwrap_or("Unnamed License")
                        .to_string(),
                    url: license_url.map(str::to_string),
                    extensions: Default::default(),
                }),
                version: version.unwrap_or("v0.0.0").to_string(),
                extensions: IndexMap::new(),
            },
            servers: vec![],
            paths: state.paths.clone(),
            components: Some(Components {
                schemas: state
                    .schemas
                    .iter()
                    .map(|(key, schema)| (key.clone(), convert_schema(schema.clone())))
                    .collect(),
                ..Default::default()
            }),
            security: None,
            tags: vec![],
            external_docs: None,
            extensions: IndexMap::new(),
        });

        state.last_build = Some(open_api.clone());
        open_api
    }
}
