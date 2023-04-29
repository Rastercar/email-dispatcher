use tokio::time;
use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry};

pub fn init(service_name: String) -> Result<(), SetGlobalDefaultError> {
    opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .with_auto_split_batch(true)
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("failed to initialize tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default()
        .with(telemetry)
        .with(tracing_subscriber::filter::LevelFilter::WARN);

    tracing::subscriber::set_global_default(subscriber)?;

    println!("[TRACER] initialized");
    Ok(())
}

// async wrapper for `opentelemetry::global::shutdown_tracer_provider()` because it might hang forever
// see: https://github.com/open-telemetry/opentelemetry-rust/issues/868
async fn shutdown_trace_provider() {
    opentelemetry::global::shutdown_tracer_provider();
}

pub async fn shutdown() {
    println!("[TRACER] shutting down");

    tokio::select! {
        _ = time::sleep(time::Duration::from_millis(500)) => {
            println!("[TRACER] gracefull shutdown failed");
        },
        _ = tokio::task::spawn_blocking(shutdown_trace_provider) => {
            println!("[TRACER] gracefull shutdown ok");
        }
    }
}
