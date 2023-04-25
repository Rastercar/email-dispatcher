use crate::{
    config,
    controller::dto::{
        events::{Email, EmailEvent},
        ses::{SesEvent, SnsNotification},
    },
    mail::mailer::MAIL_REQUEST_UUID_TAG_NAME,
    queue::server::Server,
};
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::post,
    Router,
};
use convert_case::{Case, Casing};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tracing::error;

fn get_email_event_from_json_str(body: &String) -> Result<EmailEvent, String> {
    let sns_notification = serde_json::from_str::<SnsNotification>(&body).or_else(|e| {
        Err(format!(
            "failed to parse request body to SnsNotification: {}",
            e.to_string()
        ))
    })?;

    if let Some(sub_url) = sns_notification.subscribe_url {
        let is_subscription_confirmation = sns_notification
            .notification_type
            .eq("SubscriptionConfirmation");

        if is_subscription_confirmation {
            println!("[WEB] SNS subscription confirmation link: {}", sub_url);
        }
    }

    let ses_evt = serde_json::from_str::<SesEvent>(&sns_notification.message).or_else(|e| {
        Err(format!(
            "failed to parse request body to SesEvent: {}",
            e.to_string()
        ))
    })?;

    let request_uuid = ses_evt
        .mail
        .tags
        .get(MAIL_REQUEST_UUID_TAG_NAME)
        .ok_or(format!(
            "required tag: {} not present on mail tags",
            MAIL_REQUEST_UUID_TAG_NAME
        ))?
        .first()
        .ok_or(format!(
            "required tag: {} is present but is empty",
            MAIL_REQUEST_UUID_TAG_NAME
        ))?
        .to_owned();

    let event_type = ses_evt
        .event_type
        .or(ses_evt.notification_type)
        .ok_or("failed to get event type from ses event")?
        .to_case(Case::Snake);

    let err_msg = format!("object for event of type: {} not present", event_type);

    let event = match event_type.as_str() {
        "send" => Email::send(ses_evt.send.ok_or(err_msg)?),
        "open" => Email::open(ses_evt.open.ok_or(err_msg)?),
        "click" => Email::click(ses_evt.click.ok_or(err_msg)?),
        "bounce" => Email::bounce(ses_evt.bounce.ok_or(err_msg)?),
        "reject" => Email::reject(ses_evt.reject.ok_or(err_msg)?),
        "failure" => Email::failure(ses_evt.failure.ok_or(err_msg)?),
        "delivery" => Email::delivery(ses_evt.delivery.ok_or(err_msg)?),
        "complaint" => Email::complaint(ses_evt.complaint.ok_or(err_msg)?),
        "subscription" => Email::subscription(ses_evt.subscription.ok_or(err_msg)?),
        "delivery_delay" => Email::delivery_delay(ses_evt.delivery_delay.ok_or(err_msg)?),
        _ => return Err(format!("unknown event type: {}", event_type)),
    };

    Ok(EmailEvent {
        event,
        event_type,
        request_uuid,
        mail: ses_evt.mail,
    })
}

async fn handle_ses_event(
    State(state): State<AppState>,
    body: String,
) -> Result<String, StatusCode> {
    match get_email_event_from_json_str(&body) {
        Ok(email_event) => {
            if let Err(publish_error) = state.queue_server.publish_as_json(email_event).await {
                error!("ses event publishing failed: {}", publish_error)
            }

            Ok("event handled correctly".to_owned())
        }
        Err(error) => {
            error!(error);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[derive(Clone)]
struct AppState {
    queue_server: Arc<Server>,
    aws_email_sns_subscription_arn: Option<String>,
}

/// forbids any incoming requests where the x-amz-sns-subscription-arn
/// does not match the `aws_email_sns_subscription_arn` in the application state,
/// in order to avoid potentially malicious requests from registering fake events
async fn check_sns_arn<T>(
    State(state): State<AppState>,
    req: Request<T>,
    nxt: Next<T>,
) -> Result<Response, StatusCode> {
    if state.aws_email_sns_subscription_arn.is_none() {
        return Ok(nxt.run(req).await);
    }

    if let Some(arn_header) = req.headers().get("x-amz-sns-subscription-arn") {
        let sns_header_matches = arn_header.to_str().unwrap_or("").eq(state
            .aws_email_sns_subscription_arn
            .unwrap_or("".to_owned())
            .as_str());

        if sns_header_matches {
            return Ok(nxt.run(req).await);
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

pub async fn serve(cfg: &config::AppConfig, server: Arc<Server>) {
    let state = AppState {
        queue_server: server,
        aws_email_sns_subscription_arn: cfg.aws_sns_tracking_subscription_arn.clone(),
    };

    let app = Router::new()
        .route("/ses-events", post(handle_ses_event))
        .route_layer(middleware::from_fn_with_state(state.clone(), check_sns_arn))
        .with_state(state);

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), cfg.http_port);
    println!("[WEB] listening on {}", addr);

    axum::Server::try_bind(&addr)
        .expect(format!("[WEB] failed to get address {}", addr).as_str())
        .serve(app.into_make_service())
        .await
        .expect(format!("[WEB] failed to serve app on address {}", addr).as_str())
}
