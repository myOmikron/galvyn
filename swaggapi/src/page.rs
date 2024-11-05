use crate::internals::{convert_schema, SchemaGenerator};
use crate::router::MutHandlerMeta;
use axum::http::Method;
use core::fmt;
use indexmap::IndexMap;
use openapiv3::{
    Components, Contact, Info, License, OpenAPI, Operation, PathItem, Paths, ReferenceOr,
};
use regex::Regex;
use schemars::schema::Schema;
use schemars::JsonSchema;
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, Mutex};

/// An implicit [`SwaggapiPage`] which will always contain your entire api
pub static PAGE_OF_EVERYTHING: SwaggapiPage = SwaggapiPage::new();

/// A page is a collection of api endpoints
///
/// You can think of each type implementing this as one `openapi.json`.
///
/// ## Why
///
/// This can be useful if you want to split your api into separate parts with separate openapi files.
///
/// If you don't need this, you can ignore it.
/// The [`PageOfEverything`] will be used implicitly, if you don't say otherwise.
///
/// ## How
///
/// ```rust
/// # use swaggapi::SwaggapiPage;
/// static MY_CUSTOM_API_PAGE: SwaggapiPage = SwaggapiPage::new()
///     .title("My custom subset of api endpoints");
///
/// // use &MY_CUSTOM_API_PAGE wherever an `impl SwaggapiPage` is required
/// ```
pub struct SwaggapiPage {
    title: Option<&'static str>,
    description: Option<&'static str>,
    terms_of_service: Option<&'static str>,
    contact_name: Option<&'static str>,
    contact_url: Option<&'static str>,
    contact_email: Option<&'static str>,
    license_name: Option<&'static str>,
    license_url: Option<&'static str>,
    version: Option<&'static str>,

    filename: Option<&'static str>,

    state: Mutex<Option<SwaggapiPageImpl>>,
}

/// The parts of a [`SwaggapiPage`] which are considered [internal](crate::internals)
///
/// This struct implements the actual construction of an [`OpenAPI`] document
/// combining the handlers added trough [`crate::internals::SwaggapiPageImpl::add_handler`]
/// and the metadata stored in [`SwaggapiPage`].
#[derive(Default)]
struct SwaggapiPageImpl {
    paths: Paths,

    schemas: BTreeMap<String, Schema>,

    /// Cache for the result of [`SwaggapiPage::build`]
    last_build: Option<Arc<OpenAPI>>,
}

impl fmt::Debug for SwaggapiPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SwaggapiPageBuilder")
            .finish_non_exhaustive()
    }
}

impl SwaggapiPage {
    /// Construct a new empty builder
    pub const fn new() -> Self {
        Self {
            title: None,
            description: None,
            terms_of_service: None,
            contact_name: None,
            contact_url: None,
            contact_email: None,
            license_name: None,
            license_url: None,
            version: None,
            filename: None,
            state: Mutex::new(None),
        }
    }

    /// The title of the application.
    pub const fn title(mut self, title: &'static str) -> Self {
        self.title = Some(title);
        self
    }

    /// A short description of the application.
    pub const fn description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    /// A URL to the Terms of Service for the API.
    pub const fn terms_of_service(mut self, terms: &'static str) -> Self {
        self.terms_of_service = Some(terms);
        self
    }

    /// The identifying name of the contact person/organization for the exposed API.
    pub const fn contact_name(mut self, name: &'static str) -> Self {
        self.contact_name = Some(name);
        self
    }

    /// The URL pointing to the contact information for the exposed API.
    pub const fn contact_url(mut self, url: &'static str) -> Self {
        self.contact_url = Some(url);
        self
    }

    /// The email address of the contact person/organization for the exposed API.
    pub const fn contact_email(mut self, email: &'static str) -> Self {
        self.contact_email = Some(email);
        self
    }

    /// The license name used for the API.
    pub const fn license_name(mut self, name: &'static str) -> Self {
        self.license_name = Some(name);
        self
    }

    /// A URL to the license used for the API.
    ///
    /// You should also set the `license_name`.
    pub const fn license_url(mut self, url: &'static str) -> Self {
        self.license_url = Some(url);
        self
    }

    /// The filename the page will be served as
    pub const fn filename(mut self, file: &'static str) -> Self {
        self.filename = Some(file);
        self
    }

    /// Returns the [`OpenAPI`] file
    ///
    /// The internal build process is cached (hence the `Arc`) so feel free to call this eagerly.
    pub fn openapi(&self) -> Arc<OpenAPI> {
        let SwaggapiPage {
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
        } = self;
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
                    name: license_name.unwrap_or("Unnamed License").to_string(),
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

    /// Explicitly adds a schema to this page
    ///
    /// This method's use cases are rare,
    /// because schemas are normally added implicitly with the handlers using them.
    pub fn add_schema<T: JsonSchema>(&self) -> &Self {
        let mut guard = self.state.lock().unwrap();
        let state = guard.get_or_insert_with(Default::default);
        state.last_build = None;

        SchemaGenerator::employ(&mut state.schemas, |gen| gen.generate::<T>());
        self
    }

    /// Add a handler to this api page
    pub(crate) fn add_handler(&self, handler: &MutHandlerMeta) {
        let mut guard = self.state.lock().unwrap();
        let state = guard.get_or_insert_with(Default::default);
        state.last_build = None;

        let (parameters, mut request_body, responses) =
            SchemaGenerator::employ(&mut state.schemas, |gen| {
                let mut parameters = Vec::new();
                let mut request_body = Vec::new();
                for arg in handler.handler_arguments {
                    if let Some(arg) = arg.as_ref() {
                        static PATH_PARAM_REGEX: LazyLock<Regex> =
                            LazyLock::new(|| Regex::new(r"\{[^}]*}").unwrap());
                        let path_params = PATH_PARAM_REGEX
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
            Method::GET => &mut path.get,
            Method::POST => &mut path.post,
            Method::PUT => &mut path.put,
            Method::DELETE => &mut path.delete,
            Method::HEAD => &mut path.head,
            Method::OPTIONS => &mut path.options,
            Method::PATCH => &mut path.patch,
            Method::TRACE => &mut path.trace,
            _ => unreachable!(),
        };
        *operation_mut = Some(operation);
    }
}
