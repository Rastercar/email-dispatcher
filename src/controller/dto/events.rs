use std::sync::Arc;

use chrono::{DateTime, Utc};
use lapin::publisher_confirm::PublisherConfirm;
use serde::{Deserialize, Serialize};

use crate::queue::server::Server;

#[derive(strum_macros::Display, Deserialize, Serialize)]
pub enum EmailRequestStatus {
    ERROR,
    STARTED,
    FINISHED,
    REJECTED,
}

#[derive(Deserialize, Serialize)]
pub struct EmailRequestEvent {
    pub status: EmailRequestStatus,

    pub timestamp: DateTime<Utc>,

    /// uuid of the email request this sending status update refers to
    pub request_uuid: uuid::Uuid,
}

impl EmailRequestEvent {
    pub fn started(request_uuid: uuid::Uuid) -> EmailRequestEvent {
        EmailRequestEvent {
            status: EmailRequestStatus::STARTED,
            timestamp: Utc::now(),
            request_uuid,
        }
    }

    pub fn finished(request_uuid: uuid::Uuid) -> EmailRequestEvent {
        EmailRequestEvent {
            status: EmailRequestStatus::FINISHED,
            timestamp: Utc::now(),
            request_uuid,
        }
    }

    pub fn rejected(request_uuid: uuid::Uuid) -> EmailRequestEvent {
        EmailRequestEvent {
            status: EmailRequestStatus::REJECTED,
            timestamp: Utc::now(),
            request_uuid,
        }
    }

    pub fn to_json_string(&self) -> Result<String, String> {
        serde_json::to_string(&self).or(Err("failed to deserialize EmailRequestEvent".to_owned()))
    }

    pub fn routing_key(&self) -> String {
        return format!("sending.{}", self.status.to_string().to_lowercase());
    }

    /// Publishes the self event to the mailer events exchange as JSON
    pub async fn publish(&self, rmq_server: Arc<Server>) -> Result<PublisherConfirm, String> {
        rmq_server
            .publish_event(
                self.routing_key().as_str(),
                self.to_json_string()?.as_bytes(),
            )
            .await
            .or(Err("failed to publish email event".to_owned()))
    }
}
