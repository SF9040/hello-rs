mod server;

use anyhow::{Context, Result};
use configured::Configured;
use futures::FutureExt;
use opentelemetry::{
    sdk::{propagation::TraceContextPropagator, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use serde::Deserialize;
use std::error::Error;
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub service_name: String,
    pub otlp_exporter_endpoint: String,
    pub server: server::Config,
}

pub async fn run() -> Result<()> {
    // Load configuration
    let config = Config::load();
    if let Err(e) = &config {
        eprintln!("Cannot load configuration: {e} {:?}", e.source());
    };
    let config = config?;
    let logged_config = config.clone();

    // Initialize tracing
    init_tracing(config.service_name, config.otlp_exporter_endpoint)?;

    // Log configuration
    debug!(config = debug(&logged_config), "Starting");

    // Run the server
    let server = server::run(config.server);
    info!("Started");
    server
        .inspect(|_| opentelemetry::global::shutdown_tracer_provider())
        .await
}

/// Initialize tracing
fn init_tracing(service_name: String, otlp_exporter_endpoint: String) -> Result<()> {
    // Activate trace context propagation
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .with(otel_layer(service_name, otlp_exporter_endpoint)?)
        .try_init()
        .context("Cannot initialize tracing subscriber")
}

/// Create an OpenTelemetry tracing layer
fn otel_layer<S>(
    service_name: String,
    otlp_exporter_endpoint: String,
) -> Result<impl tracing_subscriber::Layer<S>>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    let trace_config =
        opentelemetry::sdk::trace::config().with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            service_name,
        )]));

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_exporter_endpoint);

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(trace_config)
        .with_exporter(exporter)
        .install_batch(opentelemetry::runtime::Tokio)
        .context("Cannot install tracer")?;

    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}
