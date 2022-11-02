#[cfg(feature = "proxy")]
use crate::hello_tonic_client::HelloTonicClient;
use anyhow::{Context, Result};
use axum::{
    body::Body, extract::Query, http::Request, response::IntoResponse, routing::get, Router, Server,
};
#[cfg(feature = "proxy")]
use axum::{extract::State, http::StatusCode};
use opentelemetry::trace::TraceContextExt;
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{debug, field, info_span, instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    addr: IpAddr,
    port: u16,
}

impl Config {
    fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.addr, self.port)
    }
}

#[derive(Debug, Deserialize)]
struct NameQuery {
    name: Option<String>,
}

#[cfg(feature = "proxy")]
#[derive(Debug, Deserialize)]
struct ProxyQuery {
    #[serde(default)]
    proxy: bool,
}

#[cfg(feature = "proxy")]
#[derive(Debug, Clone)]
struct AppState {
    hello_tonic_client: HelloTonicClient,
}

#[cfg(feature = "proxy")]
impl AppState {
    fn new(hello_tonic_client: HelloTonicClient) -> Self {
        Self { hello_tonic_client }
    }
}

#[cfg(not(feature = "proxy"))]
pub async fn run(config: Config) -> Result<()> {
    let app = Router::new().route("/", get(hello)).layer(
        ServiceBuilder::new()
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                    let headers = request.headers();
                    info_span!("request", ?headers, trace_id = field::Empty)
                }),
            )
            .map_request(record_trace_id),
    );
    spawn_server(app, &config.socket_addr()).await
}

#[cfg(feature = "proxy")]
pub async fn run(config: Config, hello_tonic_client: HelloTonicClient) -> Result<()> {
    let state = AppState::new(hello_tonic_client);
    let app = Router::with_state(state).route("/", get(hello)).layer(
        ServiceBuilder::new()
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                    let headers = request.headers();
                    info_span!("request", ?headers, trace_id = field::Empty)
                }),
            )
            .map_request(record_trace_id),
    );
    spawn_server(app, &config.socket_addr()).await
}

async fn spawn_server<S>(app: Router<S>, socket_addr: &SocketAddr) -> Result<()>
where
    S: Send + Sync + 'static,
{
    tokio::spawn(Server::bind(socket_addr).serve(app.into_make_service()))
        .await
        .map(|server_result| server_result.context("Server completed with error"))
        .context("Server panicked")
        .and_then(|r| r)
}

#[cfg(not(feature = "proxy"))]
#[instrument]
async fn hello(Query(name_query): Query<NameQuery>) -> impl IntoResponse {
    let name = name_query
        .name
        .unwrap_or_else(|| "unknown stranger".to_string());
    local_reponse(&name)
}

#[cfg(feature = "proxy")]
#[instrument(skip(state))]
async fn hello(
    Query(name_query): Query<NameQuery>,
    Query(proxy_query): Query<ProxyQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let name = name_query.name;
    if proxy_query.proxy {
        proxy_response(name, state.hello_tonic_client).await
    } else {
        let name = name.unwrap_or_else(|| "unknown stranger".to_string());
        (StatusCode::OK, local_reponse(&name))
    }
}

#[instrument]
fn local_reponse(name: &str) -> String {
    debug!("Creating local response");
    format!("Hello, {name}!")
}

#[cfg(feature = "proxy")]
#[instrument]
async fn proxy_response(
    name: Option<String>,
    mut hello_tonic_client: HelloTonicClient,
) -> (StatusCode, String) {
    debug!("Creating proxy response");

    match hello_tonic_client
        .say_hello(name)
        .await
        .context("Cannot get response from hello-tonic")
    {
        Ok(text) => (StatusCode::OK, text),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

fn record_trace_id(request: Request<Body>) -> Request<Body> {
    let span = Span::current();

    let trace_id = span.context().span().span_context().trace_id();
    span.record("trace_id", trace_id.to_string());

    request
}
