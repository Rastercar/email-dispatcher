use lapin::message::Delivery;
use uuid::Uuid;
use validator::Validate;

use crate::{
    controller::{
        dto::{
            events::{EmailRequestFinishedEvent, EmailSendingReceivedEvent},
            input,
        },
        router::{ack_delivery, Router},
    },
    mail::mailer::SendEmailOptions,
};

impl Router {
    #[tracing::instrument(skip(self))]
    pub async fn send_email(&self, delivery: Delivery) -> Result<(), String> {
        ack_delivery(&delivery).await?;

        let send_email_in = serde_json::from_slice::<input::SendEmailIn>(&delivery.data)
            .or_else(|e| Err(format!("parse error: {:#?}", e)))?;

        let uuid = send_email_in.uuid.unwrap_or(Uuid::new_v4());

        if let Err(e) = send_email_in.validate() {
            self.server
                .publish_as_json(EmailSendingReceivedEvent::rejected(uuid, send_email_in))
                .await?;

            return Err(e.to_string());
        }

        self.server
            .publish_as_json(EmailSendingReceivedEvent::started(
                uuid,
                send_email_in.clone(),
            ))
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
                reply_to_addresses: send_email_in.reply_to_addresses,
            })
            .await?;

        self.server
            .publish_as_json(EmailRequestFinishedEvent::new(uuid))
            .await?;

        Ok(())
    }
}
