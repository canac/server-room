use serde::Deserialize;
use std::fs;

// This struct represents the config that is used by the rest of the application
#[derive(Deserialize)]
pub struct Config {
    // The directory where all of the servers are located
    servers_dir: String,
}

impl Config {
    // Read the configuration from disk
    pub fn load() -> Config {
        let config_str = fs::read_to_string("config.json").expect("Error reading configuration");
        serde_json::from_str(&config_str).expect("Error parsing JSON string")
    }

    pub fn get_servers_dir(&self) -> &str {
        self.servers_dir.as_str()
    }
}
