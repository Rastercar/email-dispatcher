use lapin::message::Delivery;
use tracing::info;

use crate::controller::{ack_delivery, dto, validation::parse_validate, Router};

impl Router {
    #[tracing::instrument]
    pub async fn send_email(&self, delivery: Delivery) -> Result<(), String> {
        ack_delivery(&delivery).await?;

        let send_email_in = parse_validate::<dto::SendEmailIn>(&delivery)?;

        self.server.publish().await;

        println!("{:#?}", send_email_in);

        info!("send email starting");

        Ok(())
    }
}
