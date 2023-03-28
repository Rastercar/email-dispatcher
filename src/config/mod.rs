use serde::Deserialize;

fn def_debug() -> bool {
    false
}

fn def_rmq_uri() -> String {
    "amqp://localhost:5672".to_string()
}

fn def_tracker_events_exchange() -> String {
    "tracker_events_topic".to_string()
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    #[serde(default = "def_debug")]
    pub debug: bool,

    #[serde(default = "def_rmq_uri")]
    pub rmq_uri: String,

    #[serde(default = "def_tracker_events_exchange")]
    pub tracker_events_exchange: String,
}

impl AppConfig {
    pub fn from_env() -> AppConfig {
        match envy::from_env::<AppConfig>() {
            Ok(config) => {
                if config.debug {
                    println!("[CFG] {:?}", config);
                }

                config
            }

            Err(error) => panic!("{:#?}", error),
        }
    }
}
