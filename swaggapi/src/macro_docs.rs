//! Documented re-exports of [`swaggapi_macro`]

/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `DELETE`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::delete;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `GET`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::get;
/// Turns a function into a documented api handler
///
/// ```rust
/// /// The first line is a `summary` available in openapi
/// ///
/// /// The entire docstring is available as `description` in openapi
/// #[swaggapi::handler(Get, "/")]
/// async fn index() -> &'static str {
///     "Hello World"
/// }
///
/// /// Deletes the entire application's state
/// ///
/// /// This endpoint is very dangerous and can only be used by admins.
/// #[swaggapi::delete("/deleteAll", tags("admin", "dangerous"))]
/// async fn delete_all() -> () {
///     // ...
/// }
/// ```
///
/// ## Arguments
/// - `method`: The HTTP method this handler should respond to
///     - **required**
///     - one of `Get`, `Post,` `Put`, `Delete`, `Head`, `Options`, `Patch`, `Trace`, for example `method = Get`
///
/// - `path`: The HTTP url this handler should respond on
///
///     Note, the [`ApiContext`](crate::ApiContext) can be used to apply a common prefix to a set of handlers.
///     - **required**
///     - a string literal, for example `path = "/"`
///
/// - `tags`: A list of tags, mostly used to group handlers
///
///     Read the [OpenAPI specification](https://swagger.io/specification/v3/) for the full details
///     - optional
///     - list of string literal, for example `tags("foo", "bar)`
///
/// ## Positional arguments
/// Since `method` and `path` are required, they can alternatively be passed as positional arguments:
/// - `#[handler(Get, "/")]`
/// - `#[handler(Get, path = "/")]`
/// - `#[handler("/", method = Get)]`
///
/// ## Variants
/// The first argument `method` can be replaced with the usage of one specialized variant of `#[handler]`:
/// - [`#[get(...)]`](get) is equivalent to `#[handler(Get, ...)]`
/// - [`#[post(...)]`](post) is equivalent to `#[handler(Post, ...)]`
/// - [`#[put(...)]`](put) is equivalent to `#[handler(Put, ...)]`
/// - [`#[delete(...)]`](delete) is equivalent to `#[handler(Delete, ...)]`
/// - [`#[head(...)]`](head) is equivalent to `#[handler(Head, ...)]`
/// - [`#[options(...)]`](options) is equivalent to `#[handler(Options, ...)]`
/// - [`#[patch(...)]`](patch) is equivalent to `#[handler(Patch, ...)]`
/// - [`#[trace(...)]`](trace) is equivalent to `#[handler(Trace, ...)]`
pub use swaggapi_macro::handler;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `HEAD`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::head;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `OPTIONS`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::options;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `PATCH`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::patch;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `POST`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::post;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `PUT`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::put;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `TRACE`,
/// for everything else please refer to [``#[handler]``](handler)
pub use swaggapi_macro::trace;
/// Derives [`SwaggapiPage`](trait@crate::SwaggapiPage) for a unit struct
///
/// ```rust
/// # use swaggapi_macro::SwaggapiPage;
/// #[derive(SwaggapiPage)]
/// #[page(title = "My custom subset of api endpoints")]
/// struct MyCustomApiPAge;
/// ```
///
/// ## Arguments
/// are passed through the helper `#[page(...)]`.
///
/// All arguments are of shape `key = "value"`
/// where the list of keys and their description
/// can be taken from [`SwaggapiPageBuilder`](crate::SwaggapiPageBuilder)'s methods.
///
/// ### Noteworthy differences from the builder:
/// - `filename` will default to the struct's identifier (followed by `.json`)
pub use swaggapi_macro::SwaggapiPage;
