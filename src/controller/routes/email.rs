use lapin::message::Delivery;
use uuid::Uuid;
use validator::Validate;

use crate::{
    controller::{
        dto::{events::EmailRequestEvent, input},
        router::{ack_delivery, Router},
    },
    mail::mailer::SendEmailOptions,
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

            println!("{:?}", e);

            return Err(e.to_string());
        }

        EmailRequestEvent::started(uuid)
            .publish(self.server.clone())
            .await?;

        self.mailer
            .send_emails(SendEmailOptions {
                uuid,
                to: send_email_in.to,
                from: send_email_in.sender,
                subject: send_email_in.subject,
                body_text: send_email_in.body_text,
                body_html: send_email_in.body_html,
                track_events: send_email_in.enable_tracking,
            })
            .await?;

        Ok(())
    }
}
