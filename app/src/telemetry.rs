use std::env;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const SERVICE_NAME: &str = "otter";

pub fn init() {
    let otel_enabled = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok();

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let fmt_layer = tracing_subscriber::fmt::layer();

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    if otel_enabled {
        let resource = Resource::builder()
            .with_attributes([
                KeyValue::new("service.name", SERVICE_NAME),
                KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            ])
            .build();

        // traces
        let span_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .build()
            .expect("failed to create OTLP span exporter");

        let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(span_exporter)
            .with_resource(resource.clone())
            .build();

        opentelemetry::global::set_tracer_provider(tracer_provider.clone());

        let otel_trace_layer =
            tracing_opentelemetry::layer().with_tracer(tracer_provider.tracer(SERVICE_NAME));

        // metrics
        let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .build()
            .expect("failed to create OTLP metric exporter");

        let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
            .with_periodic_exporter(metric_exporter)
            .with_resource(resource.clone())
            .build();

        opentelemetry::global::set_meter_provider(meter_provider.clone());

        let otel_metrics_layer = tracing_opentelemetry::MetricsLayer::new(meter_provider);

        // logs
        let log_exporter = opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .build()
            .expect("failed to create OTLP log exporter");

        let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
            .with_batch_exporter(log_exporter)
            .with_resource(resource)
            .build();

        let otel_log_layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(
            &logger_provider,
        );

        registry
            .with(otel_trace_layer)
            .with(otel_metrics_layer)
            .with(otel_log_layer)
            .init();

        tracing::info!("OpenTelemetry enabled");
    } else {
        registry.init();
    }
}
