use crate::core::re_exports::opentelemetry_otlp;
use crate::core::re_exports::tracing_opentelemetry;
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::response::Response;
use galvyn_core::middleware::SimpleGalvynMiddleware;
use galvyn_core::re_exports::opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
use galvyn_core::re_exports::opentelemetry::trace::{TraceError, TracerProvider};
use galvyn_core::re_exports::opentelemetry::{Context, Key, KeyValue, Value};
use galvyn_core::re_exports::opentelemetry_otlp::WithExportConfig;
use galvyn_core::re_exports::opentelemetry_sdk::propagation::TraceContextPropagator;
use galvyn_core::re_exports::opentelemetry_sdk::{runtime, trace, Resource};
use galvyn_core::re_exports::tracing_opentelemetry::OpenTelemetrySpanExt;
use std::ops::ControlFlow;
use tracing::{warn, Span, Subscriber};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub struct OpenTelemetrySetup {
    pub service_name: String,
    pub exporter_otlp_endpoint: String,
}
impl OpenTelemetrySetup {
    pub fn opentelemetry_layer<S: Subscriber + for<'span> LookupSpan<'span>>(
        self,
    ) -> Result<impl Layer<S>, TraceError> {
        let provider = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(self.exporter_otlp_endpoint),
            )
            .with_trace_config(
                trace::Config::default().with_resource(Resource::new([KeyValue {
                    key: Key::from_static_str("service.name"),
                    value: Value::from(self.service_name),
                }])),
            )
            .install_batch(runtime::Tokio)?;

        let tracer = provider.tracer("galvyn");

        Ok(tracing_opentelemetry::layer()
            .with_threads(false) // It's a tokio worker anyway
            .with_tracked_inactivity(false)
            .with_tracer(tracer))
    }
}

/// Checks incoming requests for opentelemetry headers and set the appropriate parent span.
#[derive(Copy, Clone, Debug, Default)]
pub struct ReceiveTracesMiddleware;
impl SimpleGalvynMiddleware for ReceiveTracesMiddleware {
    async fn pre_handler(&mut self, request: Request) -> ControlFlow<Response, Request> {
        let context = headers_to_context(request.headers());
        Span::current().set_parent(context);
        ControlFlow::Continue(request)
    }
}

/// Converts an opentelemetry context to a list of headers.
///
/// Those headers can be included in requests to preserve the opentelemetry trace.
pub fn context_to_headers(context: &Context) -> HeaderMap {
    let mut map = HeaderMap::new();
    TraceContextPropagator::new().inject_context(context, &mut HeaderMapWrite(&mut map));
    map
}

/// Reads an opentelemetry context from a list of headers.
///
/// Those headers can be received in requests to preserve the opentelemetry trace.
pub fn headers_to_context(headers: &HeaderMap) -> Context {
    TraceContextPropagator::new().extract(&HeaderMapRead(headers))
}

struct HeaderMapWrite<'a>(&'a mut HeaderMap);
impl Injector for HeaderMapWrite<'_> {
    fn set(&mut self, key: &str, value: String) {
        let Ok(name) = HeaderName::try_from(key) else {
            warn!(key, value, "Opentelemetry produced an invalid header");
            return;
        };
        let Ok(value) = HeaderValue::try_from(&value) else {
            warn!(key, value, "Opentelemetry produced an invalid header");
            return;
        };
        self.0.insert(name, value);
    }
}

struct HeaderMapRead<'a>(&'a HeaderMap);
impl Extractor for HeaderMapRead<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        let value = self.0.get(key)?;
        value
            .to_str()
            .inspect_err(|_error| {
                warn!(
                    key,
                    value = value.as_bytes(),
                    "Received invalid opentelemetry header"
                )
            })
            .ok()
    }

    fn keys(&self) -> Vec<&str> {
        warn!("Extractor::keys is not implemented");
        vec![]
    }
}
