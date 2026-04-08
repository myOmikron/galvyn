use std::ops::ControlFlow;

use axum::extract::Request;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::response::Response;
use galvyn_core::middleware::SimpleGalvynMiddleware;
use galvyn_core::re_exports::opentelemetry::propagation::Extractor;
use galvyn_core::re_exports::opentelemetry::propagation::Injector;
use galvyn_core::re_exports::opentelemetry::propagation::TextMapPropagator;
use galvyn_core::re_exports::opentelemetry::trace::TracerProvider;
use galvyn_core::re_exports::opentelemetry::Context;
use galvyn_core::re_exports::opentelemetry_otlp::ExporterBuildError;
use galvyn_core::re_exports::opentelemetry_otlp::SpanExporter;
use galvyn_core::re_exports::opentelemetry_otlp::WithExportConfig;
use galvyn_core::re_exports::opentelemetry_sdk::propagation::TraceContextPropagator;
use galvyn_core::re_exports::opentelemetry_sdk::trace::SdkTracerProvider;
use galvyn_core::re_exports::opentelemetry_sdk::Resource;
use galvyn_core::re_exports::tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing::warn;
use tracing::Span;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::core::re_exports::tracing_opentelemetry;

pub struct OpenTelemetrySetup {
    pub service_name: String,
    pub exporter_otlp_endpoint: String,
}
impl OpenTelemetrySetup {
    pub fn opentelemetry_layer<S: Subscriber + for<'span> LookupSpan<'span>>(
        self,
    ) -> Result<impl Layer<S>, ExporterBuildError> {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_endpoint(self.exporter_otlp_endpoint)
            .build()?;

        let resource = Resource::builder()
            .with_service_name(self.service_name)
            .build();

        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build();

        let tracer = provider.tracer("galvyn");

        Ok(tracing_opentelemetry::layer()
            .with_threads(false) // It's a tokio worker anyway
            .with_tracked_inactivity(false)
            .with_tracer(tracer))
    }
}

/// Checks incoming requests for opentelemetry headers and set the appropriate parent span.
#[derive(Copy, Clone, Debug, Default)]
#[deprecated(note = "This entire API has to be re-designed, because dependency changed its API.")]
pub struct ReceiveTracesMiddleware;
impl SimpleGalvynMiddleware for ReceiveTracesMiddleware {
    async fn pre_handler(&mut self, request: Request) -> ControlFlow<Response, Request> {
        let context = headers_to_context(request.headers());
        if let Err(error) = Span::current().set_parent(context) {
            warn!(%error, "Failed to set parent trace");
        }
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
