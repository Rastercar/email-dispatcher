use axum::{
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::post,
    Router,
};
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::config;

#[derive(Deserialize)]
struct SesEvent {
    username: String,
}

async fn handle_ses_event(headers: HeaderMap, body: String) {
    // TODO: parse /validate content a SesEvent json and publish it to rmq
    println!("{:?}", body);
}

#[derive(Clone)]
struct AppState {
    aws_email_sns_subscription_arn: String,
}

/// blocks any incoming requests where the x-amz-sns-subscription-arn
/// does not match the `aws_email_sns_subscription_arn` in the application state,
/// in order to avoid potentially malicious requests from registering fake events
async fn check_sns_arn<T>(
    State(state): State<AppState>,
    request: Request<T>,
    next: Next<T>,
) -> Result<Response, StatusCode> {
    if let Some(arn_header) = request.headers().get("x-amz-sns-subscription-arn") {
        let sns_header_matches = arn_header
            .to_str()
            .unwrap_or("")
            .eq(state.aws_email_sns_subscription_arn.as_str());

        if sns_header_matches {
            return Ok(next.run(request).await);
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

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3005);
    println!("[WEB] listening on {}", addr);

    axum::Server::try_bind(&addr)
        .expect(format!("[WEB] failed to get address {}", addr).as_str())
        .serve(app.into_make_service())
        .await
        .expect(format!("[WEB] failed to serve app on address {}", addr).as_str())
}
