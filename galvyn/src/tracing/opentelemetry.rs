use std::any::type_name;

use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::HeaderValue;
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
use galvyn_core::re_exports::tracing_opentelemetry::SetParentError;
use tower_http::trace::MakeSpan;
use tower_http::trace::TraceLayer;
use tracing::debug;
use tracing::trace;
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

/// Attaches remote opentelemetry traces received in the requests' headers.
///
/// It wraps a [`MakeSpan`] for [`tower_http`]'s [`TraceLayer`].
///
/// The `MakeSpan` MUST create the span without entering it.
/// This should be the case for both `tower_http` implementations.
#[derive(Copy, Clone, Debug, Default)]
pub struct AttachTraces<T>(pub T);
impl<T: MakeSpan<B>, B> MakeSpan<B> for AttachTraces<T> {
    fn make_span(&mut self, request: &axum::http::Request<B>) -> Span {
        let span = self.0.make_span(request);
        let context = headers_to_context(request.headers());
        match span.set_parent(context) {
            Ok(()) => {
                trace!("Attached remote trace to request span");
            }
            Err(SetParentError::SpanDisabled) => {
                debug!(
                    reason = "span-disabled",
                    "Can't attach remote trace to request span"
                );
            }
            Err(SetParentError::LayerNotFound) => {
                debug!(
                    reason = "layer-not-found",
                    "Can't attach remote trace to request span"
                );
            }
            Err(SetParentError::AlreadyStarted) => {
                warn!(
                    reason = "already-started",
                    explanation = format!(
                        "The `{}` which is wrapped by `AttachTraces` already started the span it created. They are not compatible.",
                        type_name::<T>()
                    ),
                    "Can't attach remote trace to request span"
                );
            }
        }
        span
    }
}

/// Extends [`TraceLayer`] and any `T: MakeSpan` with a convenience method for applying [`AttachTraces`]
///
/// (The generic `Any` is an artifact of rust's coherence check and can be ignored)
pub trait AttachTracesExt<Any> {
    /// The wrapped type
    type Result;

    /// Attach remote opentelemetry traces received in the requests' headers.
    fn attach_traces(self) -> Self::Result;
}

mod brands {
    pub struct Layer;
}

impl<A, B, C, D, E, F, G> AttachTracesExt<brands::Layer> for TraceLayer<A, B, C, D, E, F, G>
where
    B: Default,
{
    type Result = TraceLayer<A, AttachTraces<B>, C, D, E, F, G>;

    fn attach_traces(self) -> Self::Result {
        self.make_span_with(Default::default())
    }
}

impl<B, T: MakeSpan<B>> AttachTracesExt<B> for T {
    type Result = AttachTraces<T>;

    fn attach_traces(self) -> Self::Result {
        AttachTraces(self)
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
