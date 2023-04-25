use config::AppConfig;
use controller::router::Router;
use lapin::message::Delivery;
use mail::mailer::Mailer;
use queue::server::Server;
use schemars::schema_for;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use trace::tracer;

use crate::controller::dto::events::{
    EmailEvent, EmailRequestFinishedEvent, EmailSendingErrorEvent, EmailSendingReceivedEvent,
};

mod config;
mod controller {
    pub mod routes {
        pub mod default;
        pub mod email;
    }
    pub mod dto {
        pub mod events;
        pub mod input;
        pub mod ses;
    }
    pub mod router;
    pub mod validation;
}
mod mail {
    pub mod mailer;
}
mod queue {
    pub mod server;
}
mod http {
    pub mod server;
}
mod trace {
    pub mod tracer;
}
mod utils {
    pub mod errors;
}

fn gen_event_schemas() {
    let schema = schema_for!(EmailSendingReceivedEvent);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());

    let schema = schema_for!(EmailRequestFinishedEvent);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());

    let schema = schema_for!(EmailSendingErrorEvent);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());

    let schema = schema_for!(EmailEvent);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}

#[tokio::main]
async fn main() {
    println!("--------------------------------------------------");
    gen_event_schemas();
    println!("--------------------------------------------------");

    let cfg = AppConfig::from_env().expect("failed to load application config");

    tracer::init(cfg.tracer_service_name.to_owned()).expect("failed to init tracer");

    let (sender, mut receiver) = mpsc::unbounded_channel::<Delivery>();

    let server = Arc::new(Server::new(&cfg, sender));

    let http_server_ref = server.clone();
    let shutdown_server_ref = server.clone();

    let mailer = Mailer::new(&cfg, server.clone()).await;

    let router = Arc::new(Router::new(server.clone(), mailer));

    tokio::spawn(async move { server.clone().start().await });
    tokio::spawn(async move { http::server::serve(&cfg, http_server_ref).await });

    let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("failed to setup signals hook");

    tokio::spawn(async move {
        for sig in signals.forever() {
            println!("\n[APP] received signal: {}, shutting down", sig);

            tracer::shutdown().await;
            shutdown_server_ref.shutdown().await;

            std::process::exit(sig)
        }
    });

    while let Some(delivery) = receiver.recv().await {
        let router = router.clone();
        tokio::spawn(async move { router.handle_delivery(delivery).await });
    }
}
