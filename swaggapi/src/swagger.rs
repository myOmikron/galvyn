use std::borrow::Cow;
use std::sync::Arc;

use swagger_ui::UrlObject;

use crate::internals::AccessSwaggapiPageBuilder;
use crate::internals::SwaggapiPageBuilderImpl;
use crate::page::SwaggapiPageBuilder;
use crate::PageOfEverything;
use crate::SwaggapiPage;

/// Set of endpoints serving a Swagger UI and the actual openapi files
pub struct SwaggerUi {
    /// The path under which to serve the ui
    pub path: &'static str,

    /// The config to use
    ///
    /// **Note:** the `urls` field will be extended
    /// by the [`SwaggapiPage`]s you added through [`SwaggerUi::page`].
    pub config: swagger_ui::Config,

    /// Running count of pages without filenames used to generate unique names for them
    unnamed_pages: usize,
    pages: Vec<(
        &'static str,
        Cow<'static, str>,
        &'static SwaggapiPageBuilder,
    )>,
}
impl Default for SwaggerUi {
    /// Normally the swagger ui is served under `"/swagger-ui"` and contains the [`PageOfEverything`]
    fn default() -> Self {
        Self {
            path: "/swagger-ui",
            config: swagger_ui::Config::default(),
            unnamed_pages: 0,
            pages: vec![(
                "Entire API",
                Cow::Borrowed("openapi.json"),
                PageOfEverything.get_builder(),
            )],
        }
    }
}
impl SwaggerUi {
    /// Returns [`SwaggerUi::default`] but without including the [`PageOfEverything`]
    pub fn without_everything() -> Self {
        Self {
            pages: vec![],
            ..Default::default()
        }
    }

    /// Returns [`SwaggerUi::default`] with a given path
    pub fn with_path(path: &'static str) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }

    /// Adds a [`SwaggapiPage`] to the ui
    pub fn page(mut self, display_name: &'static str, page: impl SwaggapiPage) -> Self {
        let builder = page.get_builder();
        let filename = builder.filename.map(Cow::Borrowed).unwrap_or_else(|| {
            self.unnamed_pages += 1;
            Cow::Owned(format!("openapi_{}.json", self.unnamed_pages))
        });
        self.pages
            .push((display_name, filename, page.get_builder()));
        self
    }
}

#[cfg(feature = "actix")]
const _: () = {
    use actix_web::dev::AppService;
    use actix_web::dev::HttpServiceFactory;
    use actix_web::web;
    use actix_web::web::scope;
    use actix_web::web::Json;
    use actix_web::web::Redirect;
    use actix_web::HttpResponse;
    use actix_web::Responder;
    use actix_web::Route;

    impl HttpServiceFactory for SwaggerUi {
        fn register(self, app: &mut AppService) {
            let mut config = self.config;
            config.urls.extend(
                self.pages
                    .iter()
                    .map(|(page_name, file_name, _)| UrlObject::new(page_name, file_name)),
            );
            let config = Arc::new(config);

            let mut scope = scope(self.path)
                .route(
                    "/",
                    serve_static(|| Redirect::to("index.html?configUrl=config.json")),
                )
                .route("config.json", serve_static(move || Json(config)));
            for (_, file_name, builder) in self.pages {
                scope = scope.route(
                    &file_name,
                    serve_static(|| Json(SwaggapiPageBuilderImpl::build(builder))),
                );
            }
            for file_name in swagger_ui::Assets::iter() {
                if let Some(file_content) = swagger_ui::Assets::get(&file_name) {
                    scope = scope.route(
                        &file_name,
                        serve_static(|| HttpResponse::Ok().body(file_content)),
                    );
                }
            }
            scope.register(app)
        }
    }

    fn serve_static<H: FnOnce() -> R + Clone + 'static, R: Responder + 'static>(
        handler: H,
    ) -> Route {
        web::get().to(move || std::future::ready((handler.clone())()))
    }
};

#[cfg(feature = "axum")]
const _: () = {
    use axum::body::Body;
    use axum::response::IntoResponse;
    use axum::response::Redirect;
    use axum::response::Response;
    use axum::routing::MethodRouter;
    use axum::Json;
    use axum::Router;

    impl<S> From<SwaggerUi> for Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        fn from(value: SwaggerUi) -> Router<S> {
            let mut config = value.config;
            config.urls.extend(
                value
                    .pages
                    .iter()
                    .map(|(page_name, file_name, _)| UrlObject::new(page_name, file_name)),
            );
            let config = Arc::new(config);

            let normal_path = value.path.trim_end_matches('/');

            let mut router = Router::<S>::new()
                .route(
                    &format!("{normal_path}/"),
                    serve_static(|| Redirect::to("index.html?configUrl=config.json")),
                )
                .route(
                    &format!("{normal_path}/config.json"),
                    serve_static(move || Json(config)),
                );
            for (_, file_name, builder) in value.pages {
                router = router.route(
                    &format!("{normal_path}/{file_name}"),
                    serve_static(|| Json(SwaggapiPageBuilderImpl::build(builder))),
                );
            }
            for file_name in swagger_ui::Assets::iter() {
                if let Some(file_content) = swagger_ui::Assets::get(&file_name) {
                    router = router.route(
                        &format!("{normal_path}/{file_name}"),
                        serve_static(|| Response::new(Body::from(file_content))),
                    );
                }
            }

            router
        }
    }

    fn serve_static<S, H, R>(handler: H) -> MethodRouter<S>
    where
        S: Clone,
        H: FnOnce() -> R + Clone,
        R: IntoResponse,
        S: Send + Sync + 'static,
        H: Send + Sync + 'static,
        R: Send + 'static,
    {
        MethodRouter::new().get(move || std::future::ready((handler.clone())()))
    }
};
