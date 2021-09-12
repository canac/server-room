use super::Server;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub servers_dir: String,
    pub servers: Vec<Server>,
}

impl Config {
    // Load the configuration from a string
    pub fn load_config() -> Config {
        // Load the configuration file contents
        let config = fs::read_to_string("config.json").expect("Error reading configuration");
        serde_json::from_str(&config).expect("Error parsing JSON string")
    }

    // Write the configuration to disk
    pub fn flush_config(self: &Config) {
        fs::write(
            "config.json",
            serde_json::to_string_pretty(self).expect("Error stringifying config to JSON"),
        )
        .expect("Error writing configuration")
    }
}
