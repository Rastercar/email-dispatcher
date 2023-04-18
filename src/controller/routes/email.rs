use lapin::message::Delivery;
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::controller::{
    ack_delivery,
    dto::{events::EmailRequestEvent, input},
    Router,
};

impl Router {
    #[tracing::instrument]
    pub async fn send_email(&self, delivery: Delivery) -> Result<(), String> {
        ack_delivery(&delivery).await?;

        let send_email_in = serde_json::from_slice::<input::SendEmailIn>(&delivery.data)
            .or_else(|e| Err(format!("parse error: {:#?}", e)))?;

        let uuid = send_email_in.uuid.unwrap_or(Uuid::new_v4());

        if let Err(e) = send_email_in.validate() {
            EmailRequestEvent::rejected(uuid)
                .publish(self.server.clone())
                .await?;

            return Err(e.to_string());
        }

        EmailRequestEvent::started(uuid)
            .publish(self.server.clone())
            .await?;

        println!("{:#?}", send_email_in);

        info!("send email starting");

        Ok(())
    }
}
