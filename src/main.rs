use aws_sdk_sesv2::{config::Region, Client, Error};
use config::AppConfig;
use lapin::message::Delivery;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
mod config;
mod controller;
mod queue;
mod trace;

mod utils {
    pub mod errors;
}

async fn send_message(
    client: &Client,
    from: &str,
    subject: &str,
    message: &str,
) -> Result<(), Error> {
    let dest = Destination::builder()
        .to_addresses("rastercar.tests.002@gmail.com")
        .build();

    let subject_content = Content::builder().data(subject).charset("UTF-8").build();

    let body_content = Content::builder().data(message).charset("UTF-8").build();

    let body = Body::builder().html(body_content).build();

    let msg = Message::builder()
        .subject(subject_content)
        .body(body)
        .build();

    let email_content = EmailContent::builder().simple(msg).build();

    client
        .send_email()
        .from_email_address(from)
        .destination(dest)
        .content(email_content)
        .send()
        .await?;

    println!("Email sent to list");

    Ok(())
}

#[tokio::main]
async fn main() {
    println!();

    let shared_config = aws_config::from_env()
        .region(Region::new("us-east-1"))
        .load()
        .await;

    let client = Client::new(&shared_config);

    send_message(
        &client,
        "rastercar.tests.001@gmail.com",
        "subject !",
        "<h1>message</h1>",
    )
    .await
    .unwrap();

    let cfg = AppConfig::from_env().expect("failed to load application config");

    trace::init(cfg.tracer_service_name).expect("failed to init tracer");

    let (sender, mut reciever) = unbounded_channel::<Delivery>();

    let options = queue::Options {
        uri: cfg.rmq_uri,
        queue: cfg.rmq_queue,
        consumer_tag: cfg.rmq_consumer_tag,
        email_events_exchange: cfg.email_events_exchange,
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
