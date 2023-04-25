use lapin::{
    message::Delivery,
    options::{
        BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, QueueDeclareOptions,
    },
    publisher_confirm::PublisherConfirm,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};
use serde::Serialize;
use std::{thread, time};
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tokio_stream::StreamExt;

use crate::{config, utils::errors};

static ERR_EMPTY_CHANNEL: &str = "channel not set";
static ERR_PUBLISH_CONFIRM: &str = "failed to confirm publishing";

#[derive(Debug)]
pub struct Options {
    pub uri: String,
    pub queue: String,
    pub consumer_tag: String,
    pub email_events_exchange: String,
}

#[derive(Debug)]
pub struct Server {
    options: Options,
    channel: RwLock<Option<Channel>>,
    connection: RwLock<Option<Connection>>,
    sender: UnboundedSender<Delivery>,
}

pub trait Routable {
    fn routing_key(&self) -> String;
}

impl Server {
    pub fn new(cfg: &config::AppConfig, sender: UnboundedSender<Delivery>) -> Server {
        let options = Options {
            uri: cfg.rmq_uri.to_owned(),
            queue: cfg.rmq_queue.to_owned(),
            consumer_tag: cfg.rmq_consumer_tag.to_owned(),
            email_events_exchange: cfg.rmq_email_events_exchange.to_owned(),
        };

        Server {
            sender: sender,
            options: options,
            channel: RwLock::new(None),
            connection: RwLock::new(None),
        }
    }

    pub async fn start(&self) {
        loop {
            if let Err(err) = self.run().await {
                println!("[RMQ] connection error: {}", err)
            }

            thread::sleep(time::Duration::from_secs(5));
            println!("[RMQ] reconnecting");
        }
    }

    pub async fn shutdown(&self) {
        println!("[RMQ] closing channel");
        if let Some(chan) = self.channel.read().await.as_ref() {
            if let Err(chan_close_err) = chan.close(200, "user shutdown").await {
                println!("[RMQ] failed to close channel: {}", chan_close_err)
            }
        }

        println!("[RMQ] closing connection");
        if let Some(conn) = self.connection.read().await.as_ref() {
            if let Err(conn_close_err) = conn.close(200, "user shutdown").await {
                println!("[RMQ] failed to close connection: {}", conn_close_err)
            }
        }
    }

    async fn run(&self) -> Result<(), lapin::Error> {
        let props = ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio);

        let connection = Connection::connect(&self.options.uri, props).await?;
        println!("[RMQ] connected");

        let channel = connection.create_channel().await?;
        println!("[RMQ] channel created");

        errors::exit_on_err(
            channel
                .exchange_declare(
                    &self.options.email_events_exchange,
                    ExchangeKind::Topic,
                    ExchangeDeclareOptions {
                        passive: false,
                        durable: true,
                        auto_delete: false,
                        internal: false,
                        nowait: false,
                    },
                    FieldTable::default(),
                )
                .await,
        );
        println!("[RMQ] events exchange declared");

        errors::exit_on_err(
            channel
                .queue_declare(
                    &self.options.queue,
                    QueueDeclareOptions {
                        nowait: false,
                        passive: false,
                        durable: true,
                        exclusive: false,
                        auto_delete: false,
                    },
                    FieldTable::default(),
                )
                .await,
        );
        println!("[RMQ] mailer queue declared");

        let mut consumer = errors::exit_on_err(
            channel
                .basic_consume(
                    &self.options.queue,
                    &self.options.consumer_tag,
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await,
        );
        println!("[RMQ] mailer queue consumer started");

        *self.connection.write().await = Some(connection);
        *self.channel.write().await = Some(channel);

        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    // the sender channel should be open for the entirety of the programn
                    self.sender.send(delivery).expect("sender channel closed");
                }
                Err(err) => {
                    println!("[RMQ] mailer queue consumer stopped due to error: {}", err);
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    pub async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &[u8],
        properties: BasicProperties,
    ) -> Result<PublisherConfirm, String> {
        self.channel
            .read()
            .await
            .as_ref()
            .ok_or(ERR_EMPTY_CHANNEL)?
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                payload,
                properties,
            )
            .await
            .or(Err(ERR_PUBLISH_CONFIRM.to_owned()))
    }

    pub async fn publish_email_event_json<T>(&self, event: T) -> Result<PublisherConfirm, String>
    where
        T: Serialize + Routable,
    {
        let json = serde_json::to_string(&event).or(Err("failed to serialize event".to_owned()))?;

        self.publish(
            &self.options.email_events_exchange,
            event.routing_key().as_str(),
            json.as_bytes(),
            BasicProperties::default().with_content_type("application/json".into()),
        )
        .await
    }
}
