use std::collections::HashMap;

use serde::Deserialize;

use validator::Validate;

use super::validation::email_vec;

#[derive(Debug, Validate, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmailRecipient {
    /// recipient email address
    #[validate(email)]
    email: String,

    // TODO: study how to pass/validate a map[string]string here
    /// An array of email adresses to send the email to and the
    /// replacements to use on the email html for that email address, eg:
    ///
    /// ```
    /// { email: "jhon@gmail.com", replacements: { "name": "jhon" } }
    /// ```
    replacements: HashMap<String, String>,
}

#[derive(Debug, Validate, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailIn {
    // TODO: use ?
    // TODO: validate is uuid
    // https://github.com/uuid-rs/uuid
    /// A unique identifier for the email sending request, this is so the client can store this on
    /// his side and use this identifier on future requests, such as getting metrics for this uuid
    uuid: Option<String>,

    // TODO: test validation allows named <name> emails and ignore None values
    /// The RFC5322 email address to be used to send the email, if None the service default address is used
    #[validate(email)]
    sender: Option<String>,

    // TODO: test nested validation is called
    /// List of recipients for the email
    #[validate]
    to: Vec<EmailRecipient>,

    // TODO: validate size of strings in vec ?
    /// Tags to store in the email, eg: ("marketing", "alert", "sample_offer")
    tags: Vec<String>,

    // TODO: test validation of strings in vect
    /// List of email adresses to show on the email reply-to options, only makes
    /// sense if at least one email address different than the sender is used
    #[validate(custom = "email_vec")]
    reply_to_adresses: Option<Vec<String>>,

    subject_text: String,

    body_html: String,

    /// Optional email text content: displayed on clients that do not support Html
    body_text: Option<String>,
}
