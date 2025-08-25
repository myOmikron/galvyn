use galvyn_core::re_exports::serde::ser::{SerializeMap, Serializer};
use galvyn_core::re_exports::time::OffsetDateTime;
use galvyn_core::stuff::schema::SchemaDateTime;
use opentelemetry::global::ObjectSafeSpan;
use opentelemetry::trace::{TraceContextExt, TraceError, TracerProvider};
use opentelemetry::{Key, KeyValue, Value};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace, Resource};
use reqwest::Url;
use std::time::Duration;
use std::{fmt, io, mem};
use tracing::{warn, Event, Span, Subscriber};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_serde::SerdeMapVisitor;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, MakeWriter};
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

/// [`Format`](tracing_subscriber::fmt::format::Format) for `tracing_subscriber::fmt` layer.
///
/// It formats each event as its own self-contained flat JSON log line,
/// similar to the [`Json`](tracing_subscriber::fmt::format::Json) format.
///
/// It has at least the following keys:
/// - `service_name`
/// - `timestamp`
/// - `level`
/// - `trace_id`
/// - `span_id`
/// - `target`
///
/// It may also have the following keys:
/// - `message`
/// - `filename`
/// - `line_number`
/// - `span_name`
///
/// Additionally, it may have any custom key-value pair defined for the event.
#[derive(Debug, Clone)]
pub struct FlatJson {
    pub service_name: String,
}

impl<S, N> FormatEvent<S, N> for FlatJson
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let ts = SchemaDateTime(OffsetDateTime::now_utc());
        let meta = event.metadata();

        let mut visit = || {
            let mut outer_serializer = serde_json::Serializer::new(WriteAdaptor {
                fmt_write: &mut writer,
            });

            let mut serializer = outer_serializer.serialize_map(None)?;
            serializer.serialize_entry("service_name", self.service_name.as_str())?;
            serializer.serialize_entry("timestamp", &ts)?;
            serializer.serialize_entry("level", meta.level().to_string().as_str())?;

            let current_span = Span::current();
            let otel_context = current_span.context().span().span_context().clone();
            serializer.serialize_entry("trace_id", &otel_context.trace_id().to_string())?;
            serializer.serialize_entry("span_id", &otel_context.span_id().to_string())?;

            let mut visitor = SerdeMapVisitor::new(serializer);
            event.record(&mut visitor);
            serializer = visitor.take_serializer()?;

            serializer.serialize_entry("target", meta.target())?;
            if let Some(filename) = meta.file() {
                serializer.serialize_entry("filename", filename)?;
            }
            if let Some(line_number) = meta.line() {
                serializer.serialize_entry("line_number", &line_number)?;
            }
            if let Some(metadata) = current_span.metadata() {
                serializer.serialize_entry("span_name", metadata.name())?;
            }
            serializer.end()
        };

        visit().map_err(|_| fmt::Error)?;
        writeln!(writer)
    }
}

#[derive(Clone)]
pub struct AlloyWriter {
    client: reqwest::Client,
    target_url: Url,
    body: Vec<u8>,
}

impl AlloyWriter {
    pub fn new(base_url: Url) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            target_url: base_url.join("/loki/api/v1/raw")?,
            body: Vec::new(),
        })
    }
}

impl MakeWriter<'_> for AlloyWriter {
    type Writer = AlloyWriter;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

impl io::Write for AlloyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.body.extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for AlloyWriter {
    fn drop(&mut self) {
        let request = self
            .client
            .post(self.target_url.clone())
            .body(mem::take(&mut self.body))
            .send();
        tokio::spawn(async move {
            let response = match request.await {
                Ok(res) => res,
                Err(_err) => {
                    return;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let text = response.text().await.unwrap_or_default();
                warn!(
                    status = ?status,
                    text = text.as_str(),
                    "HTTP error while writing to Alloy",
                );
            }
        });
    }
}

/// Bridge between [`fmt::Write`] and [`io::Write`].
struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn fmt::Write,
}
impl io::Write for WriteAdaptor<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // TODO: fix buf might not be valid utf8
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.fmt_write.write_str(s).map_err(io::Error::other)?;
        Ok(s.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
