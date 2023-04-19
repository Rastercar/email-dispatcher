use std::collections::HashMap;

use serde::Deserialize;

use validator::Validate;

use super::super::validation::email_vec;

#[derive(Debug, Validate, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmailRecipient {
    /// recipient email address
    #[validate(email)]
    pub email: String,

    /// An array of email adresses to send the email to and the
    /// replacements to use on the email html for that email address, eg:
    ///
    /// ```
    /// { email: "jhon@gmail.com", replacements: { "name": "jhon" } }
    /// ```
    pub replacements: HashMap<String, String>,
}

#[derive(Debug, Validate, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailIn {
    /// A unique identifier for the email sending request, this is so the client can store this on
    /// his side and use this identifier on future requests, such as getting metrics for this uuid
    pub uuid: Option<uuid::Uuid>,

    // TODO: make this work with RFC5322 emails
    /// The RFC5322 email address to be used to send the email, if None the service default address is used
    #[validate(email)]
    pub sender: Option<String>,

    // TODO: validate contains at least one
    /// List of recipients for the email
    #[validate]
    pub to: Vec<EmailRecipient>,

    // TODO: validate size of strings in vec ?
    /// Tags to store in the email, eg: ("marketing", "alert", "sample_offer")
    pub tags: Vec<String>,

    /// List of email adresses to show on the email reply-to options, only makes
    /// sense if at least one email address different than the sender is used
    #[validate(custom = "email_vec")]
    pub reply_to_adresses: Option<Vec<String>>,

    pub subject: String,

    pub body_html: Option<String>,

    /// Optional email text content: displayed on clients that do not support Html
    pub body_text: Option<String>,

    /// If tracking for email events such as clicks and opens should be enabled
    #[serde(default)]
    pub enable_tracking: bool,
}
