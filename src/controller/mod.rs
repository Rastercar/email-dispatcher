use crate::queue;
use lapin::{message::Delivery, options::BasicNackOptions, types::ShortString};
use std::sync::Arc;

mod routes {
    pub mod email;
}

fn get_delivery_type(delivery: &Delivery) -> String {
    delivery
        .properties
        .kind()
        .clone()
        .unwrap_or(ShortString::from("unknown"))
        .to_string()
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
            _ => handle_delivery_without_corresponding_rpc(delivery).await,
        };

        if let Err(err) = handler_res {
            todo!("log error, {}", err)
        }
    }
}

pub fn create_ack_nack_error_string(delivery: Delivery) -> Result<(), String> {
    Err(format!(
        "error acking/nacking, delivery with tag: {} of type: {}",
        delivery.delivery_tag,
        get_delivery_type(&delivery)
    ))
}

pub async fn handle_delivery_without_corresponding_rpc(delivery: Delivery) -> Result<(), String> {
    delivery
        .nack(BasicNackOptions::default())
        .await
        .or(create_ack_nack_error_string(delivery))
}
