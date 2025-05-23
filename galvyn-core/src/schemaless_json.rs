/// JSON Extractor / Response which doesn't require [`JsonSchema`](schemars::JsonSchema)
///
/// It is an alternative to [`Json`] (re-exported from `axum`)
/// which ignores whether `T` implements [`JsonSchema`](schemars::JsonSchema).
#[derive(Copy, Clone, Debug)]
pub struct SchemalessJson<T>(pub T);

mod axum_impls {
    use axum::Json;
    use axum::extract::FromRequest;
    use axum::extract::Request;
    use axum::response::IntoResponse;
    use axum::response::Response;

    use crate::SchemalessJson;

    impl<T, S: Sync> FromRequest<S> for SchemalessJson<T>
    where
        Json<T>: FromRequest<S>,
    {
        type Rejection = <Json<T> as FromRequest<S>>::Rejection;

        async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
            <Json<T> as FromRequest<S>>::from_request(req, state)
                .await
                .map(|Json(t)| SchemalessJson(t))
        }
    }

    impl<T> IntoResponse for SchemalessJson<T>
    where
        Json<T>: IntoResponse,
    {
        fn into_response(self) -> Response {
            Json(self.0).into_response()
        }
    }
}

mod galvyn_impls {
    use axum::http::StatusCode;
    use mime::Mime;
    use schemars::schema::Schema;

    use crate::SchemalessJson;
    use crate::handler::request_body::RequestBody;
    use crate::handler::request_body::ShouldBeRequestBody;
    use crate::handler::response_body::ResponseBody;
    use crate::handler::response_body::ShouldBeResponseBody;
    use crate::schema_generator::SchemaGenerator;

    impl<T> ShouldBeRequestBody for SchemalessJson<T> {}
    impl<T> RequestBody for SchemalessJson<T> {
        fn body(_generator: &mut SchemaGenerator) -> (Mime, Option<Schema>) {
            (mime::APPLICATION_JSON, None)
        }
    }

    impl<T> ShouldBeResponseBody for SchemalessJson<T> {}
    impl<T> ResponseBody for SchemalessJson<T> {
        fn body(
            _generator: &mut SchemaGenerator,
        ) -> Vec<(StatusCode, Option<(Mime, Option<Schema>)>)> {
            vec![(StatusCode::OK, Some((mime::APPLICATION_JSON, None)))]
        }
    }
}
