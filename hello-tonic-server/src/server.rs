mod proto {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("hello");
}

use self::proto::{
    hello_server::{Hello, HelloServer},
    HelloRequest, HelloResponse,
};
use anyhow::{Context, Result};
use opentelemetry::{global, propagation::Extractor, trace::TraceContextExt};
use serde::Deserialize;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::time::sleep;
use tonic::{
    codegen::http::{header::HeaderName, HeaderMap, Request as HttpRequest},
    transport::{Body, Server},
    Request, Response, Status,
};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{error, field, info_span, instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    addr: IpAddr,
    port: u16,
    delay_millis: u64,
}

impl Config {
    fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.addr, self.port)
    }
}

pub async fn run(config: Config) -> Result<()> {
    let delay = Duration::from_millis(config.delay_millis);
    let this_hello = ThisHello { delay };

    let app = Server::builder()
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_grpc().make_span_with(|request: &HttpRequest<Body>| {
                        let headers = request.headers();
                        info_span!("request", ?headers, trace_id = field::Empty)
                    }),
                )
                .map_request(set_span_parent)
                .map_request(record_trace_id),
        )
        .add_service(HelloServer::new(this_hello));

    app.serve(config.socket_addr())
        .await
        .context("Server completed with error")
}

#[derive(Debug, Clone)]
struct ThisHello {
    delay: Duration,
}

#[tonic::async_trait]
impl Hello for ThisHello {
    #[instrument(skip(self), err)]
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        do_something_before(self.delay).await;
        let name = match request.into_inner().name {
            name if name.is_empty() => "unknown stranger".to_string(),
            name => name,
        };
        let text = format!("Hello, {name}!");
        Ok(Response::new(HelloResponse { text }))
    }
}

#[instrument]
async fn do_something_before(duration: Duration) -> () {
    sleep(duration).await
}

// Trace context propagation

struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| match value.to_str() {
            Ok(value) => Some(value),
            Err(error) => {
                error!(
                    %error,
                    "Cannot convert header value to valid ASCII",
                );
                None
            }
        })
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(HeaderName::as_str).collect()
    }
}

fn set_span_parent(request: HttpRequest<Body>) -> HttpRequest<Body> {
    let span = Span::current();

    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(request.headers()))
    });
    span.set_parent(parent_context);

    request
}

fn record_trace_id(request: HttpRequest<Body>) -> HttpRequest<Body> {
    let span = Span::current();

    let trace_id = span.context().span().span_context().trace_id();
    span.record("trace_id", trace_id.to_string());

    request
}
