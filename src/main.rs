use config::AppConfig;
use controller::router::Router;
use lapin::message::Delivery;
use mail::mailer::Mailer;
use queue::server::Server;
use std::sync::Arc;
use tokio::sync::mpsc;

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
mod trace;
mod utils {
    pub mod errors;
}

#[tokio::main]
async fn main() {
    let cfg = AppConfig::from_env().expect("failed to load application config");

    trace::init(cfg.tracer_service_name.to_owned()).expect("failed to init tracer");

    let (sender, mut reciever) = mpsc::unbounded_channel::<Delivery>();

    let server = Arc::new(Server::new(&cfg, sender));

    let mailer = Mailer::new(&cfg, server.clone()).await;

    let router = Arc::new(Router::new(server.clone(), mailer));

    tokio::spawn(async move { server.start().await });

    tokio::spawn(async move { http::server::serve(&cfg).await });

    while let Some(delivery) = reciever.recv().await {
        let router = router.clone();
        tokio::spawn(async move { router.handle_delivery(delivery).await });
    }

    trace::shutdown();
}
