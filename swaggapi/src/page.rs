use core::fmt;
use std::sync::Arc;
use std::sync::Mutex;

use openapiv3::OpenAPI;
use schemars::JsonSchema;

use crate::internals::AccessSwaggapiPageBuilder;
use crate::internals::SwaggapiPageBuilderImpl;

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
/// 1. Use [`#[derive(SwaggapiPage)]`](macro@crate::SwaggapiPage) on a unit struct to create a new api page
/// 2. Pass the unit struct to [`ApiContext::page`](crate::ApiContext::page) to add some endpoints
/// 3. Pass the unit struct to [`SwaggerUi::page`](crate::SwaggerUi::page) to expose it in the swagger ui
pub trait SwaggapiPage: AccessSwaggapiPageBuilder {
    /// Returns the [`OpenAPI`] file
    ///
    /// The internal build process is cached (hence the `Arc`) so feel free to call this eagerly.
    fn openapi(&self) -> Arc<OpenAPI>;

    /// Explicitly adds a schema to this page
    ///
    /// This method's use cases are rare,
    /// because schemas are normally added implicitly with the handlers using them.
    fn add_schema<S: JsonSchema>(&self) -> &Self;
}
impl<P: AccessSwaggapiPageBuilder> SwaggapiPage for P {
    fn openapi(&self) -> Arc<OpenAPI> {
        SwaggapiPageBuilderImpl::build(self.get_builder())
    }

    fn add_schema<T: JsonSchema>(&self) -> &Self {
        SwaggapiPageBuilderImpl::add_schema::<T>(self.get_builder());
        self
    }
}

/// An implicit [`SwaggapiPage`] which will always contain your entire api
pub struct PageOfEverything;
/// "Manual expansion" of [`derive(SwaggapiPage)`](crate::SwaggapiPage)
impl AccessSwaggapiPageBuilder for PageOfEverything {
    fn get_builder(&self) -> &'static SwaggapiPageBuilder {
        static BUILDER: SwaggapiPageBuilder = SwaggapiPageBuilder::new();
        &BUILDER
    }
}

/// Builder used to configure an api page
///
/// This type is either used internally by the [`#[derive(SwaggapiPage)]`](macro@crate::SwaggapiPage) macro
/// or can be used manually to create an [`SwaggapiPage`](trait@SwaggapiPage) without macro:
///
/// ```rust
/// # use swaggapi::SwaggapiPageBuilder;
/// static MY_CUSTOM_API_PAGE: SwaggapiPageBuilder = SwaggapiPageBuilder::new()
///     .title("My custom subset of api endpoints");
///
/// // use &MY_CUSTOM_API_PAGE wherever an `impl SwaggapiPage` is required
/// ```
///
/// This example is semantically equivalent to the one from [`#[derive(SwaggapiPage)]`](macro@crate::SwaggapiPage).
pub struct SwaggapiPageBuilder {
    pub(crate) title: Option<&'static str>,
    pub(crate) description: Option<&'static str>,
    pub(crate) terms_of_service: Option<&'static str>,
    pub(crate) contact_name: Option<&'static str>,
    pub(crate) contact_url: Option<&'static str>,
    pub(crate) contact_email: Option<&'static str>,
    pub(crate) license_name: Option<&'static str>,
    pub(crate) license_url: Option<&'static str>,
    pub(crate) version: Option<&'static str>,

    pub(crate) filename: Option<&'static str>,

    pub(crate) state: Mutex<Option<SwaggapiPageBuilderImpl>>,
}

impl fmt::Debug for SwaggapiPageBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SwaggapiPageBuilder")
            .finish_non_exhaustive()
    }
}

impl SwaggapiPageBuilder {
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
}
