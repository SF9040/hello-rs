mod proto {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("hello");
}

use self::proto::{hello_client::HelloClient, HelloRequest};
use anyhow::{Context, Result};
use configured::Configured;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub endpoint: String,
}

pub async fn run() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Invoke server
    let mut client = HelloClient::connect(config.endpoint.clone())
        .await
        .context(format!("Cannot connect to {}", config.endpoint))?;
    let response = client
        .say_hello(HelloRequest {
            name: "".to_string(),
        })
        .await
        .context("HelloRequest failed")?;
    println!("{}", response.into_inner().text);

    Ok(())
}
