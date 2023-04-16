use config::AppConfig;
use lapin::message::Delivery;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

mod config;
mod controller;
mod queue;
mod trace;
mod utils {
    pub mod errors;
}

#[tokio::main]
async fn main() {
    let cfg = AppConfig::from_env().expect("failed to load application config");

    trace::init(cfg.tracer_service_name).expect("failed to init tracer");

    let (sender, mut reciever) = unbounded_channel::<Delivery>();

    let options = queue::Options {
        uri: cfg.rmq_uri,
        queue: cfg.rmq_queue,
        consumer_tag: cfg.rmq_consumer_tag,
    };

    let server = Arc::new(queue::Server::new(options, sender));
    let router = Arc::new(controller::Router::new(server.clone()));

    tokio::spawn(async move { server.start().await });

    while let Some(delivery) = reciever.recv().await {
        let copy = router.clone();
        tokio::spawn(async move { copy.handle_delivery(delivery).await });
    }

    trace::shutdown();
}
