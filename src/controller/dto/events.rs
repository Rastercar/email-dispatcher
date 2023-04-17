#[derive(strum_macros::Display)]
pub enum EmailRequestStatus {
    ERROR,
    STARTED,
    FINISHED,
    REJECTED,
}

struct EmailRequestStatusEvent {
    status: EmailRequestStatus,

    //
    timestamp: String,

    /// uuid of the email request this sending status update refers to
    request_uuid: String,
}
