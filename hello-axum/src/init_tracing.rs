use anyhow::{Context, Result};
use opentelemetry::{
    global, runtime,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, Sampler, ShouldSample},
        Resource,
    },
    trace::{
        Link, OrderMap, SamplingDecision, SamplingResult, SpanKind, TraceContextExt, TraceId,
        TraceState,
    },
    InstrumentationLibrary, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use rand::Rng;
use serde::Deserialize;
use tracing::Subscriber;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt, EnvFilter, Layer,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    service_name: String,
    otlp_exporter_endpoint: String,
    sampling_probability: f64,
}

/// Initialize tracing
pub fn init_tracing(config: Config) -> Result<()> {
    // Activate trace context propagation
    global::set_text_map_propagator(TraceContextPropagator::new());

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().json())
        .with(otel_layer(
            config.service_name,
            config.otlp_exporter_endpoint,
            config.sampling_probability,
        )?)
        .try_init()
        .context("Cannot initialize tracing subscriber")
}

/// Create an OpenTelemetry tracing layer
fn otel_layer<S>(
    service_name: String,
    otlp_exporter_endpoint: String,
    sampling_probability: f64,
) -> Result<impl Layer<S>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let trace_config = trace::config()
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            service_name,
        )]))
        .with_sampler(Sampler::ParentBased(Box::new(ProbabilitySampler(
            sampling_probability,
        ))));

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_exporter_endpoint);

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(trace_config)
        .with_exporter(exporter)
        .install_batch(runtime::Tokio)
        .context("Cannot install tracer")?;

    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

#[derive(Debug, Clone, Copy)]
struct ProbabilitySampler(f64);

impl ShouldSample for ProbabilitySampler {
    fn should_sample(
        &self,
        parent_cx: Option<&opentelemetry::Context>,
        _trace_id: TraceId,
        _name: &str,
        _span_kind: &SpanKind,
        _attributes: &OrderMap<opentelemetry::Key, opentelemetry::Value>,
        _links: &[Link],
        _instrumentation_library: &InstrumentationLibrary,
    ) -> SamplingResult {
        let decision = parent_cx
            .filter(|parent_cx| parent_cx.has_active_span())
            .map_or(
                {
                    if rand::thread_rng().gen_range(0.0..=1.0) <= self.0 {
                        SamplingDecision::RecordAndSample
                    } else {
                        SamplingDecision::Drop
                    }
                },
                |parent_cx| {
                    if parent_cx.span().span_context().is_sampled() {
                        SamplingDecision::RecordAndSample
                    } else {
                        SamplingDecision::Drop
                    }
                },
            );
        SamplingResult {
            decision,
            attributes: Vec::new(),
            trace_state: match parent_cx {
                Some(ctx) => ctx.span().span_context().trace_state().clone(),
                None => TraceState::default(),
            },
        }
    }
}
