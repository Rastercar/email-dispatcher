use config::AppConfig;
use controller::router::Router;
use lapin::message::Delivery;
use mail::mailer::Mailer;
use queue::server::Server;
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::sync::Arc;
use tokio::sync::mpsc;
use trace::tracer;

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

#[tokio::main]
async fn main() {
    let cfg = AppConfig::from_env().expect("failed to load application config");

    tracer::init(cfg.tracer_service_name.to_owned()).expect("failed to init tracer");

    let (sender, mut receiver) = mpsc::unbounded_channel::<Delivery>();

    let server = Arc::new(Server::new(&cfg, sender));
    let server_ref = server.clone();

    let mailer = Mailer::new(&cfg, server.clone()).await;

    let router = Arc::new(Router::new(server.clone(), mailer));

    tokio::spawn(async move { server.clone().start().await });
    tokio::spawn(async move { http::server::serve(&cfg).await });

    let mut signals = Signals::new(&[SIGINT]).expect("failed to setup signals hook");

    tokio::spawn(async move {
        for sig in signals.forever() {
            println!("\n[APP] received signal: {}, shutting down", sig);

            tracer::shutdown().await;
            server_ref.shutdown().await;

            std::process::exit(sig)
        }
    });

    while let Some(delivery) = receiver.recv().await {
        let router = router.clone();
        tokio::spawn(async move { router.handle_delivery(delivery).await });
    }
}
