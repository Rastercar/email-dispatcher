//! DTOS for all the events that are fired by this service

use super::input::SendEmailIn;
use crate::queue::server::Routable;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(strum_macros::Display, Deserialize, Serialize)]
pub enum EmailRequestStatus {
    #[strum(serialize = "started")]
    STARTED,

    #[strum(serialize = "rejected")]
    REJECTED,
}

/// informs that a request has been received by this service and its status
#[derive(Deserialize, Serialize)]
pub struct EmailSendingReceivedEvent {
    pub timestamp: DateTime<Utc>,

    pub status: EmailRequestStatus,

    pub request_uuid: Uuid,

    pub request: SendEmailIn,
}

impl EmailSendingReceivedEvent {
    pub fn started(request_uuid: uuid::Uuid, request: SendEmailIn) -> EmailSendingReceivedEvent {
        EmailSendingReceivedEvent {
            request,
            request_uuid,
            timestamp: Utc::now(),
            status: EmailRequestStatus::STARTED,
        }
    }

    pub fn rejected(request_uuid: uuid::Uuid, request: SendEmailIn) -> EmailSendingReceivedEvent {
        EmailSendingReceivedEvent {
            request,
            request_uuid,
            timestamp: Utc::now(),
            status: EmailRequestStatus::REJECTED,
        }
    }
}

impl Routable for EmailSendingReceivedEvent {
    fn routing_key(&self) -> String {
        match self.status {
            EmailRequestStatus::STARTED => "sending.started".to_string(),
            EmailRequestStatus::REJECTED => "sending.rejected".to_string(),
        }
    }
}

/// informs that all the emails for a request have been fired to the AWS servers, this does not mean
/// the emails have all been successfully fired, much less that they reached the recipients inboxes
#[derive(Deserialize, Serialize)]
pub struct EmailRequestFinishedEvent {
    pub timestamp: DateTime<Utc>,

    pub request_uuid: uuid::Uuid,
}

impl Routable for EmailRequestFinishedEvent {
    fn routing_key(&self) -> String {
        "sending.finished".to_string()
    }
}

impl EmailRequestFinishedEvent {
    pub fn new(request_uuid: uuid::Uuid) -> EmailRequestFinishedEvent {
        EmailRequestFinishedEvent {
            timestamp: Utc::now(),
            request_uuid,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct EmailSendingErrorEvent {
    pub timestamp: DateTime<Utc>,

    pub request_uuid: Uuid,

    pub error: String,
}

impl EmailSendingErrorEvent {
    pub fn new(request_uuid: uuid::Uuid, error: String) -> EmailSendingErrorEvent {
        EmailSendingErrorEvent {
            error,
            timestamp: Utc::now(),
            request_uuid,
        }
    }
}

impl Routable for EmailSendingErrorEvent {
    fn routing_key(&self) -> String {
        "sending.error".to_string()
    }
}
