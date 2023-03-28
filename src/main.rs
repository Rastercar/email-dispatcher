mod queue;
use config::AppConfig;
use queue::RmqMessage;
use server::h02;
mod config;
use tokio::sync::mpsc::unbounded_channel;

mod decoder;
mod server;

#[tokio::main]
async fn main() {
    let (sender, reciever) = unbounded_channel::<RmqMessage>();

    let config = AppConfig::from_env();

    let rabbitmq_options = queue::RmqServerOptions {
        uri: config.rmq_uri,
        tracker_events_exchange: config.tracker_events_exchange,
    };

    let mut rmq = queue::RmqServer::new(rabbitmq_options, reciever)
        .await
        .unwrap();

    tokio::spawn(async move {
        rmq.start_message_consumer().await;
    });

    server::start_tcp_listener("127.0.0.1:3003", sender, h02::stream_handler)
        .await
        .unwrap();
}
