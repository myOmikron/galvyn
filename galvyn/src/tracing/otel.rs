use opentelemetry::trace::{TraceError, TracerProvider};
use opentelemetry::{Key, KeyValue, Value};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace, Resource};
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub mod re_exports {
    pub use opentelemetry;
    pub use opentelemetry_sdk;
    pub use tracing_opentelemetry;
}

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
