use std::{thread, time};

use lapin::{
    message::Delivery,
    options::{BasicConsumeOptions, BasicPublishOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};

use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tokio_stream::StreamExt;

use crate::utils::errors;

#[derive(Debug)]
pub struct Options {
    pub uri: String,
    pub queue: String,
    pub consumer_tag: String,
}

#[derive(Debug)]
pub struct Server {
    options: Options,
    channel: RwLock<Option<Channel>>,
    connection: RwLock<Option<Connection>>,
    sender: UnboundedSender<Delivery>,
}

impl Server {
    pub fn new(opts: Options, sender: UnboundedSender<Delivery>) -> Server {
        Server {
            options: opts,
            sender: sender,
            channel: RwLock::new(None),
            connection: RwLock::new(None),
        }
    }

    pub async fn start(&self) {
        loop {
            match self.run().await {
                Ok(_) => println!("[RMQ] consumer stream closed without returning an error"),
                Err(err) => println!("[RMQ] connection error: {}", err),
            }

            thread::sleep(time::Duration::from_secs(5));
            println!("[RMQ] reconecting");
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

        let queue_opts = lapin::options::QueueDeclareOptions {
            nowait: false,
            passive: false,
            durable: true,
            exclusive: false,
            auto_delete: false,
        };

        errors::exit_on_err(
            channel
                .queue_declare(&self.options.queue, queue_opts, FieldTable::default())
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

    // TODO: remove mocked publishing
    pub async fn publish(&self) {
        self.channel
            .read()
            .await
            .as_ref()
            // TODO: rm unwrap !
            .unwrap()
            .basic_publish(
                "",
                "dev_test_queue",
                BasicPublishOptions::default(),
                b"Hello world!",
                BasicProperties::default(),
            )
            .await
            .unwrap()
            .await
            .unwrap();
    }
}
