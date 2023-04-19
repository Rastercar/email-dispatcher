use crate::{config, controller::dto::input, queue::server};
use aws_sdk_sesv2::{
    config::Region,
    operation::send_email::builders::SendEmailFluentBuilder,
    types::{Body, Content, Destination, EmailContent, Message},
    Client,
};
use handlebars::Handlebars;
use std::sync::Arc;
use uuid::Uuid;

/// see: https://docs.aws.amazon.com/ses/latest/APIReference/API_SendEmail.html
static MAX_RECIPIENTS_PER_SEND_EMAIL_OP: usize = 50;

pub struct SendEmailOptions {
    pub to: Vec<input::EmailRecipient>,
    pub from: Option<String>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,

    /// If the email to be sent should have tracking for (click, delivery, report, send and open events)
    /// this changes how the email is fired in the following ways:
    ///
    /// a call with the sendEmail op is sent to SES for every email addreess in the `to` field, so we can properly
    /// track events to the recipient level, this is slower and more expensive as this triggers SNS events.
    ///
    /// the configuration set used to fire the emails
    pub track_events: bool,
}

#[derive(Debug)]
pub struct Mailer {
    pub server: Arc<server::Server>,
    pub aws_client: Client,
    pub default_sender: String,
    pub aws_ses_tracking_config_set: String,
}

// TODO: rate limiting ?
async fn send_email_to_ses(email_req: SendEmailFluentBuilder) {
    email_req.send().await;
}

impl Mailer {
    pub async fn new(cfg: &config::AppConfig, server: Arc<server::Server>) -> Mailer {
        let aws_cfg = aws_config::from_env()
            .region(Region::new(cfg.aws_region.to_owned()))
            .load()
            .await;

        Mailer {
            server,
            aws_client: Client::new(&aws_cfg),
            default_sender: cfg.app_default_email_sender.to_owned(),
            aws_ses_tracking_config_set: cfg.aws_ses_tracking_config_set.to_owned(),
        }
    }

    fn to_utf8_content(&self, input: impl Into<String>) -> Content {
        Content::builder().data(input).charset("UTF-8").build()
    }

    // TODO: document me !
    pub fn schedule_email_sendings(&self, options: SendEmailOptions) -> Result<(), String> {
        let html = options.body_html.unwrap_or("".to_owned());
        let text = options.body_text.unwrap_or("".to_owned());
        let subject = self.to_utf8_content(options.subject);

        let from = options.from.unwrap_or(self.default_sender.to_owned());

        let config_set = if options.track_events {
            Some(self.aws_ses_tracking_config_set.to_owned())
        } else {
            None
        };

        let (recipients_without_replacements, recipients_with_replacements): (_, Vec<_>) = options
            .to
            .clone()
            .into_iter()
            .partition(|recipient| recipient.replacements.is_empty());

        if !recipients_with_replacements.is_empty() {
            let mut reg = Handlebars::new();

            let temp_id = Uuid::new_v4().to_string();

            let template_registered = reg.register_template_string(&temp_id, &html).is_ok();

            for recipient in recipients_with_replacements {
                let recipient_html = if template_registered {
                    reg.render(&temp_id, &recipient.replacements)
                        .unwrap_or(html.clone())
                } else {
                    html.clone()
                };

                let body = Body::builder()
                    .html(self.to_utf8_content(recipient_html))
                    .text(self.to_utf8_content(text.clone()))
                    .build();

                let msg = Message::builder()
                    .subject(subject.clone())
                    .body(body)
                    .build();

                let email_content = EmailContent::builder().simple(msg).build();

                let dest = Destination::builder().to_addresses(recipient.email).build();

                let builder = self
                    .aws_client
                    .send_email()
                    .from_email_address(from.clone())
                    .destination(dest)
                    .set_configuration_set_name(config_set.clone())
                    .content(email_content.clone());

                tokio::spawn(async move { send_email_to_ses(builder).await });
            }
        }

        if !recipients_without_replacements.is_empty() {
            let chunk_size = if options.track_events {
                1
            } else {
                MAX_RECIPIENTS_PER_SEND_EMAIL_OP
            };

            for recipient_chunk in options.to.chunks(chunk_size) {
                let chunk_emails = recipient_chunk
                    .to_vec()
                    .iter()
                    .map(|e| e.email.to_owned())
                    .collect();

                let body = Body::builder()
                    .html(self.to_utf8_content(html.clone()))
                    .text(self.to_utf8_content(text.clone()))
                    .build();

                let msg = Message::builder()
                    .subject(subject.clone())
                    .body(body)
                    .build();

                let email_content = EmailContent::builder().simple(msg).build();

                let dest = Destination::builder()
                    .set_to_addresses(Some(chunk_emails))
                    .build();

                let builder = self
                    .aws_client
                    .send_email()
                    .from_email_address(from.clone())
                    .destination(dest)
                    .set_configuration_set_name(config_set.clone())
                    .content(email_content.clone());

                tokio::spawn(async move { send_email_to_ses(builder).await });
            }
        }

        Ok(())
    }
}
