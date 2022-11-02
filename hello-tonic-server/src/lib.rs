mod init_tracing;
mod server;

use anyhow::Result;
use configured::Configured;
use futures::FutureExt;
use init_tracing::init_tracing;
use serde::Deserialize;
use std::error::Error;
use tracing::{debug, info};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub init_tracing: init_tracing::Config,
    pub server: server::Config,
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

    // Run the server
    let server = server::run(config.server);
    info!("Started");
    server
        .inspect(|_| opentelemetry::global::shutdown_tracer_provider())
        .await
}
