use serde::Deserialize;

fn def_debug() -> bool {
    false
}

fn def_rmq_uri() -> String {
    "amqp://localhost:5672".to_string()
}

fn def_rmq_queue() -> String {
    "mailer".to_string()
}

fn def_rmq_consumer_tag() -> String {
    "mailer_service_consumer".to_string()
}

fn def_tracer_service_name() -> String {
    "mailer".to_string()
}

fn def_email_events_exchange() -> String {
    "email_events".to_string()
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    #[serde(default = "def_debug")]
    pub debug: bool,

    #[serde(default = "def_tracer_service_name")]
    pub tracer_service_name: String,

    #[serde(default = "def_rmq_uri")]
    pub rmq_uri: String,

    #[serde(default = "def_rmq_queue")]
    pub rmq_queue: String,

    #[serde(default = "def_rmq_consumer_tag")]
    pub rmq_consumer_tag: String,

    #[serde(default = "def_email_events_exchange")]
    pub email_events_exchange: String,
}

impl AppConfig {
    pub fn from_env() -> Result<AppConfig, envy::Error> {
        match envy::from_env::<AppConfig>() {
            Ok(config) => {
                if config.debug {
                    println!("[CFG] {:?}", config);
                }

                Ok(config)
            }

            Err(error) => Err(error),
        }
    }
}
