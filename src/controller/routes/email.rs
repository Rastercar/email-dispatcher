use lapin::{message::Delivery, options::BasicAckOptions};
use tracing::info;

use crate::controller::{create_ack_nack_error_string, Router};

impl Router {
    #[tracing::instrument]
    pub async fn send_email(&self, delivery: Delivery) -> Result<(), String> {
        self.server.publish().await;

        info!("send email starting");

        delivery
            .ack(BasicAckOptions::default())
            .await
            .or(create_ack_nack_error_string(delivery))
    }
}
