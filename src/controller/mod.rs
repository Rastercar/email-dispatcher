use crate::queue;
use lapin::{
    message::Delivery,
    options::{BasicAckOptions, BasicNackOptions},
    types::ShortString,
};
use routes::default;
use std::sync::Arc;

pub mod validation;
mod dto {
    pub mod events;
    pub mod input;
}
mod routes {
    pub mod default;
    pub mod email;
}

#[derive(Debug)]
pub struct Router {
    server: Arc<queue::Server>,
}

impl Router {
    pub fn new(server: Arc<queue::Server>) -> Router {
        Router { server }
    }

    pub async fn handle_delivery(&self, delivery: Delivery) {
        let handler_res = match get_delivery_type(&delivery).as_str() {
            "sendEmail" => self.send_email(delivery).await,
            _ => default::handle_delivery_without_corresponding_rpc(delivery).await,
        };

        if let Err(err) = handler_res {
            // TODO: RM ME!
            println!("err -> {}", err);

            // TODO: trace/log error on jaeger !?
            todo!("log error, {}", err)
        }
    }
}

fn get_delivery_type(delivery: &Delivery) -> String {
    delivery
        .properties
        .kind()
        .clone()
        .unwrap_or(ShortString::from("unknown"))
        .to_string()
}

pub async fn ack_delivery(delivery: &Delivery) -> Result<(), String> {
    delivery
        .ack(BasicAckOptions::default())
        .await
        .or(Err(create_ack_nack_error_string(&delivery)))
}

pub async fn nack_delivery(delivery: &Delivery) -> Result<(), String> {
    delivery
        .nack(BasicNackOptions::default())
        .await
        .or(Err(create_ack_nack_error_string(&delivery)))
}

pub fn create_ack_nack_error_string(delivery: &Delivery) -> String {
    format!(
        "error acking/nacking, delivery with tag: {} of type: {}",
        delivery.delivery_tag,
        get_delivery_type(delivery)
    )
}
