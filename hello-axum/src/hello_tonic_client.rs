mod proto {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("hello");
}

use self::proto::{hello_client::HelloClient, HelloRequest};
use anyhow::{Context, Result};
use opentelemetry::propagation::Injector;
use serde::Deserialize;
use std::str::FromStr;
use tonic::{
    metadata::{MetadataKey, MetadataMap, MetadataValue},
    transport::Endpoint,
    Request, Status,
};
use tracing::{error, instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone)]
pub struct HelloTonicClient {
    endpoint: Endpoint,
}

impl HelloTonicClient {
    pub async fn new(config: Config) -> Result<Self> {
        let endpoint = Endpoint::from_str(config.endpoint.as_str()).context(format!(
            "Cannot create endpoint for happy-hacking-grpc from {}",
            &config.endpoint
        ))?;
        Ok(Self { endpoint })
    }

    #[instrument(skip(self))]
    pub async fn say_hello(&mut self, name: Option<String>) -> Result<String> {
        let channel = self
            .endpoint
            .connect()
            .await
            .context(format!("Cannot connect to endpoint {:?}", &self.endpoint))?;

        let request = match name {
            Some(name) => HelloRequest { name },
            None => HelloRequest::default(),
        };

        HelloClient::with_interceptor(channel, inject_trace_context)
            .say_hello(request)
            .await
            .context("Error invoking SayHello")
            .map(|response| response.into_inner().text)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    endpoint: String,
}

// Trace context propagation

struct MetadataInjector<'a>(&'a mut MetadataMap);

impl Injector for MetadataInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        match MetadataKey::from_bytes(key.as_bytes()) {
            Ok(name) => match MetadataValue::try_from(&value) {
                Ok(value) => {
                    self.0.insert(name, value);
                }
                Err(e) => error!(error = display(e), "Cannot parse {value} as metadata value",),
            },
            Err(e) => error!(error = display(e), "Cannot parse {key} as metadata value",),
        }
    }
}

fn inject_trace_context(mut request: Request<()>) -> Result<Request<()>, Status> {
    opentelemetry::global::get_text_map_propagator(|propagator| {
        let context = Span::current().context();
        propagator.inject_context(&context, &mut MetadataInjector(request.metadata_mut()))
    });
    Ok(request)
}
