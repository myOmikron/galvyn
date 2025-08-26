use galvyn_core::re_exports::time::format_description::well_known::Rfc3339;
use galvyn_core::re_exports::time::OffsetDateTime;

use opentelemetry::trace::TraceContextExt;
use reqwest::Url;
use std::fmt::Debug;
use std::time::Duration;
use std::{fmt, io, mem};
use tracing::field::Field;
use tracing::{warn, Event, Span, Subscriber};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, MakeWriter};
use tracing_subscriber::registry::LookupSpan;

pub mod otel;

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
        #[derive(Default)]
        struct JsonVisitor(serde_json::Map<String, serde_json::Value>);
        impl JsonVisitor {
            fn insert(&mut self, key: impl ToString, value: impl Into<serde_json::Value>) {
                self.0.insert(key.to_string(), value.into());
            }
            fn finish(self) -> serde_json::Value {
                serde_json::Value::Object(self.0)
            }
        }
        impl tracing::field::Visit for JsonVisitor {
            fn record_f64(&mut self, field: &Field, value: f64) {
                self.insert(field.name(), value);
            }

            fn record_i64(&mut self, field: &Field, value: i64) {
                self.insert(field.name(), value);
            }

            fn record_u64(&mut self, field: &Field, value: u64) {
                self.insert(field.name(), value);
            }

            fn record_bool(&mut self, field: &Field, value: bool) {
                self.insert(field.name(), value);
            }

            fn record_str(&mut self, field: &Field, value: &str) {
                self.insert(field.name(), value);
            }

            fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
                self.insert(field.name(), format!("{value:?}"));
            }
        }

        let meta = event.metadata();

        let mut json = JsonVisitor::default();
        json.insert(
            "timestamp",
            OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| "ERROR".to_string()),
        );
        json.insert("level", meta.level().to_string());
        json.insert("target", meta.target().to_string());
        if let Some(filename) = meta.file() {
            json.insert("filename", filename.to_string());
        }
        if let Some(line_number) = meta.line() {
            json.insert("line_number", line_number);
        }

        let current_span = Span::current();
        let otel_context = current_span.context().span().span_context().clone();
        json.insert("trace_id", otel_context.trace_id().to_string());
        json.insert("span_id", otel_context.span_id().to_string());
        json.insert("service_name", self.service_name.clone());
        if let Some(metadata) = current_span.metadata() {
            json.insert("span_name", metadata.name().to_string());
        }

        event.record(&mut json);

        writeln!(writer, "{}", json.finish())
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
