#[cfg(feature = "proxy")]
mod hello_tonic_client;
mod init_tracing;
mod server;

#[cfg(feature = "proxy")]
use crate::hello_tonic_client::HelloTonicClient;
use anyhow::Result;
use configured::Configured;
use futures::FutureExt;
use init_tracing::init_tracing;
use opentelemetry::global;
use serde::Deserialize;
use std::error::Error;
use tracing::{debug, info};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub init_tracing: init_tracing::Config,
    pub server: server::Config,
    #[cfg(feature = "proxy")]
    pub hello_tonic_client: hello_tonic_client::Config,
}

pub async fn run() -> Result<()> {
    // Load configuration
    let config = Config::load();
    if let Err(error) = &config {
        eprintln!("Cannot load configuration: {error} {:?}", error.source());
    };
    let config = config?;

    // Initialize tracing
    init_tracing(config.clone().init_tracing)?;

    // Log configuration
    debug!(?config, "Starting");

    // Create hello-tonic client
    #[cfg(feature = "proxy")]
    let hello_tonic_client = HelloTonicClient::new(config.hello_tonic_client).await?;

    // Run the server
    #[cfg(not(feature = "proxy"))]
    let server = server::run(config.server);
    #[cfg(feature = "proxy")]
    let server = server::run(config.server, hello_tonic_client);
    info!("Started");
    server.inspect(|_| global::shutdown_tracer_provider()).await
}
