use crate::{mail::mailer::Mailer, queue::server};
use lapin::{
    message::Delivery,
    options::{BasicAckOptions, BasicNackOptions},
    types::ShortString,
};
use std::sync::Arc;

use super::routes::default;

#[derive(Debug)]
pub struct Router {
    pub server: Arc<server::Server>,
    pub mailer: Mailer,
}

impl Router {
    pub fn new(server: Arc<server::Server>, mailer: Mailer) -> Router {
        Router { server, mailer }
    }

    pub async fn handle_delivery(&self, delivery: Delivery) {
        let handler_res = match get_delivery_type(&delivery).as_str() {
            "sendEmail" => self.send_email(delivery).await,
            _ => default::handle_delivery_without_corresponding_rpc(delivery).await,
        };

        // TODO: trace/log error on jaeger !?
        if let Err(err) = handler_res {
            // TODO: RM ME!
            println!("err -> {}", err);
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
