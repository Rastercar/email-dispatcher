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
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tracing::error;

async fn handle_ses_event(
    State(state): State<AppState>,
    body: String,
) -> Result<String, StatusCode> {
    let bad_request = StatusCode::BAD_REQUEST;

    let sns_notification = serde_json::from_str::<SnsNotification>(&body).or(Err(bad_request))?;
    let ses_evt =
        serde_json::from_str::<SesEvent>(&sns_notification.message).or(Err(bad_request))?;

    let request_uuid = ses_evt
        .mail
        .tags
        .get(MAIL_REQUEST_UUID_TAG_NAME)
        .ok_or(bad_request)?
        .first()
        .ok_or(bad_request)?
        .to_owned();

    let event_type = ses_evt
        .event_type
        .or(ses_evt.notification_type)
        .ok_or(bad_request)?;

    let event = match event_type.as_str() {
        "send" => Email::SEND(ses_evt.send.ok_or(bad_request)?),
        "open" => Email::OPEN(ses_evt.open.ok_or(bad_request)?),
        "click" => Email::CLICK(ses_evt.click.ok_or(bad_request)?),
        "bounce" => Email::BOUNCE(ses_evt.bounce.ok_or(bad_request)?),
        "reject" => Email::REJECT(ses_evt.reject.ok_or(bad_request)?),
        "failure" => Email::FAILURE(ses_evt.failure.ok_or(bad_request)?),
        "delivery" => Email::DELIVERY(ses_evt.delivery.ok_or(bad_request)?),
        "complaint" => Email::COMPLAINT(ses_evt.complaint.ok_or(bad_request)?),
        "subscription" => Email::SUBSCRIPTION(ses_evt.subscription.ok_or(bad_request)?),
        "deliveryDelay" => Email::DELIVERY_DELAY(ses_evt.delivery_delay.ok_or(bad_request)?),
        _ => return Err(bad_request),
    };

    let email_event = EmailEvent {
        event,
        request_uuid,
        mail: ses_evt.mail,
        event_type: event_type.to_owned(),
    };

    if let Err(publish_error) = state.queue_server.publish_as_json(email_event).await {
        error!("ses event publishing failed: {}", publish_error)
    }

    Ok("event handled correctly".to_owned())
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
