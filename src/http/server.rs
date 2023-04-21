use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::post,
    Router,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::{config, controller::dto::ses};

async fn handle_ses_event(body: String) {
    // TODO: test all basic SES events and publish a simplified version of them to rabbitmq

    let sns_notification = serde_json::from_str::<ses::SnsNotification>(&body)
        .or_else(|e| Err(format!("parse error: {:#?}", e)))
        .unwrap();

    let ses_event = serde_json::from_str::<ses::SesEvent>(&sns_notification.message)
        .or_else(|e| Err(format!("parse error: {:#?}", e)))
        .unwrap();

    println!("==================================");
    println!(
        "-------- ses_event: {} --------",
        ses_event.event_type.clone().unwrap_or("??".to_owned())
    );
    println!("{:#?}", ses_event);
    println!("==================================");
}

#[derive(Clone)]
struct AppState {
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

pub async fn serve(cfg: &config::AppConfig) {
    let state = AppState {
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
