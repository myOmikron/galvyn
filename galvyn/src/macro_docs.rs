//! Documented re-exports of [`galvyn_macros`]

/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `DELETE`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::delete;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `GET`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::get;
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
pub use galvyn_macros::handler;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `HEAD`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::head;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `OPTIONS`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::options;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `PATCH`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::patch;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `POST`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::post;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `PUT`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::put;
/// Turns a function into a documented api handler
///
/// Unlike `#[handler]` it uses the http method `TRACE`,
/// for everything else please refer to [``#[handler]``](handler)
pub use galvyn_macros::trace;
