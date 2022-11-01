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
    if let Err(e) = &config {
        eprintln!("Cannot load configuration: {e} {:?}", e.source());
    };
    let config = config?;
    let logged_config = config.clone();

    // Initialize tracing
    init_tracing(config.init_tracing)?;

    // Log configuration
    debug!(config = debug(&logged_config), "Starting");

    // Run the server
    let server = server::run(config.server);
    info!("Started");
    server
        .inspect(|_| opentelemetry::global::shutdown_tracer_provider())
        .await
}
