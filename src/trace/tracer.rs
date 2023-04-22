use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry};

pub fn init(service_name: String) -> Result<(), SetGlobalDefaultError> {
    opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .install_simple()
        .expect("failed to initialize tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);

    tracing::subscriber::set_global_default(subscriber)?;

    println!("[TRACER] initialized");
    Ok(())
}

pub fn shutdown() {
    println!("[TRACER] shutting down");
    opentelemetry::global::shutdown_tracer_provider();
}
