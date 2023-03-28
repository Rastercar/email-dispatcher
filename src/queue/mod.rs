use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};
use tokio::sync::mpsc::UnboundedReceiver;

pub struct RmqServerOptions {
    // rabbitmq uri
    pub uri: String,

    // the exchange to send tracker events to
    pub tracker_events_exchange: String,
}

pub struct RmqServer {
    options: RmqServerOptions,
    channel: Channel,
    reciever: UnboundedReceiver<RmqMessage>,
}

#[derive(Debug)]
pub struct RmqMessage {
    pub routing_key: String,
    pub body: String, // TODO: must be bytes ?
}

impl RmqServer {
    pub async fn new(
        options: RmqServerOptions,
        reciever: UnboundedReceiver<RmqMessage>,
    ) -> Result<RmqServer, lapin::Error> {
        let conn_options = ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio);

        let connection = Connection::connect(&options.uri, conn_options).await?;
        println!("[RMQ] connected");

        let channel = connection.create_channel().await?;
        println!("[RMQ] channel created");

        channel
            .exchange_declare(
                &options.tracker_events_exchange,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    nowait: false,
                    passive: false,
                    durable: true,
                    internal: false,
                    auto_delete: false,
                },
                FieldTable::default(),
            )
            .await?;
        println!("[RMQ] tracker events exchange created");

        Ok(RmqServer {
            options,
            channel,
            reciever,
        })
    }

    // TODO: document me properly once the scope is set
    /// listens to the self.reciever channel until its dropped
    pub async fn start_message_consumer(&mut self) {
        println!("[RMQ] listening to messages to send");

        loop {
            let x = self.reciever.recv().await;

            match x {
                None => {
                    println!("[RMQ] listener stopped");
                    return;
                }
                Some(rmq_msg) => {
                    println!(
                        "got routing_key = {}, msg = {}",
                        rmq_msg.routing_key, rmq_msg.body
                    );

                    self.channel
                        .basic_publish(
                            &self.options.tracker_events_exchange,
                            "ds",
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
        }
    }
}
