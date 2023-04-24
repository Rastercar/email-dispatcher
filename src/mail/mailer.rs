use crate::{config, controller::dto::input, queue::server};
use aws_sdk_sesv2::{
    client::customize::Response,
    config::Region,
    error::SdkError,
    operation::send_email::{builders::SendEmailFluentBuilder, SendEmailError, SendEmailOutput},
    types::{Body, Content, Destination, EmailContent, Message, MessageTag},
    Client,
};
use governor::{
    clock::{QuantaClock, QuantaInstant},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota,
};
use handlebars::Handlebars;
use std::{num::NonZeroU32, sync::Arc, thread, time};
use tokio::task::JoinSet;
use tracing::Instrument;
use uuid::Uuid;

/// see: https://docs.aws.amazon.com/ses/latest/APIReference/API_SendEmail.html
static MAX_RECIPIENTS_PER_SEND_EMAIL_OP: usize = 50;

static MAX_EMAIL_RETRY_ATTEMPT: u8 = 4;

static RETRY_ATTEMPTS_INTERVAL: u8 = 5;

#[derive(Debug)]
pub struct SendEmailOptions {
    pub to: Vec<input::EmailRecipient>,

    pub from: Option<String>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,

    /// Uuid of the email request, used to publish error/finished events when all the deliveries for the request finish
    pub uuid: Uuid,

    /// If the email to be sent should have tracking for (click, delivery, report, send and open events)
    /// this changes how the email is fired in the following ways:
    ///
    /// a call with the sendEmail op is sent to SES for every email address in the `to` field, so we can properly
    /// track events to the recipient level, this is slower and more expensive as this triggers SNS events.
    ///
    /// the configuration set used to fire the emails
    pub track_events: bool,
}

type RateLimiter =
    governor::RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

#[derive(Debug)]
pub struct Mailer {
    pub server: Arc<server::Server>,
    pub aws_client: Client,
    pub rate_limiter: Arc<RateLimiter>,
    pub default_sender: String,
    pub aws_ses_tracking_config_set: String,
}

#[tracing::instrument]
async fn send_with_rate_limiter(
    rate_limiter: Arc<RateLimiter>,
    send_email_op: SendEmailFluentBuilder,
) -> Result<SendEmailOutput, SdkError<SendEmailError, Response>> {
    rate_limiter.until_ready().await;

    let mut result = send_email_op.clone().send().await;
    let mut attempt = 1;

    while attempt < MAX_EMAIL_RETRY_ATTEMPT && !result.is_ok() {
        attempt += 1;

        thread::sleep(time::Duration::from_secs(RETRY_ATTEMPTS_INTERVAL.into()));

        rate_limiter.until_ready().await;
        result = send_email_op.clone().send().await;
    }

    // TODO:
    // if result.is_err fire a error event for the recipients in this OP

    result
}

impl Mailer {
    pub async fn new(cfg: &config::AppConfig, server: Arc<server::Server>) -> Mailer {
        let aws_cfg = aws_config::from_env()
            .region(Region::new(cfg.aws_region.to_owned()))
            .load()
            .await;

        let time_limit = NonZeroU32::new(cfg.aws_ses_max_emails_per_second).unwrap();
        let rate_limiter = governor::RateLimiter::direct(Quota::per_second(time_limit));

        Mailer {
            server,
            rate_limiter: Arc::new(rate_limiter),
            aws_client: Client::new(&aws_cfg),
            default_sender: cfg.app_default_email_sender.to_owned(),
            aws_ses_tracking_config_set: cfg.aws_ses_tracking_config_set.to_owned(),
        }
    }

    fn to_utf8_content(&self, input: impl Into<String>) -> Content {
        Content::builder().data(input).charset("UTF-8").build()
    }

    /// Sends the emails for all the recipients in parallel, passing uuid to the email tags.
    ///
    /// Each recipient with non empty replacements have the `body_html` {{}} tags
    /// replaced by the recipients replacements. Emails are send individually for
    /// every recipient with replacements or for every recipient if `track_events` is true.
    ///
    /// this future resolves once all the emails have been sent
    #[tracing::instrument(skip(self))]
    pub async fn send_emails(&self, options: SendEmailOptions) -> Result<(), String> {
        let html = options.body_html.unwrap_or("".to_owned());
        let text = options.body_text.unwrap_or("".to_owned());
        let subject = self.to_utf8_content(options.subject);

        let uuid_str = options.uuid.to_string();

        let from = options.from.unwrap_or(self.default_sender.to_owned());

        let config_set = if options.track_events {
            Some(self.aws_ses_tracking_config_set.to_owned())
        } else {
            None
        };

        let (recipients_with_replacements, recipients_without_replacements): (_, Vec<_>) = options
            .to
            .into_iter()
            .partition(|recipient| recipient.has_replacements());

        let mut send_email_tasks = JoinSet::new();

        let email_id_tag = MessageTag::builder()
            .name("raster-email-id")
            .value(uuid_str.clone())
            .build();

        if !recipients_with_replacements.is_empty() {
            let mut reg = Handlebars::new();

            let template_registered = reg.register_template_string(&uuid_str, &html).is_ok();

            for recipient in recipients_with_replacements {
                let recipient_html = if template_registered {
                    reg.render(&uuid_str, &recipient.replacements)
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

                let dest = Destination::builder()
                    .to_addresses(recipient.email.clone())
                    .build();

                send_email_tasks.spawn(
                    send_with_rate_limiter(
                        self.rate_limiter.clone(),
                        self.aws_client
                            .send_email()
                            .from_email_address(from.clone())
                            .destination(dest)
                            .email_tags(email_id_tag.clone())
                            .set_configuration_set_name(config_set.clone())
                            .content(email_content.clone()),
                    )
                    .instrument(tracing::Span::current()),
                );
            }
        }

        if !recipients_without_replacements.is_empty() {
            let chunk_size = if options.track_events {
                1
            } else {
                MAX_RECIPIENTS_PER_SEND_EMAIL_OP
            };

            for recipient_chunk in recipients_without_replacements.chunks(chunk_size) {
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

                send_email_tasks.spawn(
                    send_with_rate_limiter(
                        self.rate_limiter.clone(),
                        self.aws_client
                            .send_email()
                            .from_email_address(from.clone())
                            .destination(dest)
                            .email_tags(email_id_tag.clone())
                            .set_configuration_set_name(config_set.clone())
                            .content(email_content.clone()),
                    )
                    .instrument(tracing::Span::current()),
                );
            }
        }

        while let Some(_) = send_email_tasks.join_next().await {}

        Ok(())
    }
}
