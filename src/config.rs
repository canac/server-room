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
        let config_str = fs::read_to_string("config.json").expect("Error reading configuration");
        let mut config: Config =
            serde_json::from_str(&config_str).expect("Error parsing JSON string");
        config
            .servers
            .sort_by(|server1, server2| server1.get_weight().cmp(&server2.get_weight()).reverse());
        config
    }

    // Write the configuration to disk
    pub fn flush_config(self: &Config) {
        fs::write(
            "config.json",
            serde_json::to_string_pretty(self).expect("Error stringifying config to JSON"),
        )
        .expect("Error writing configuration")
    }

    // Permanently add a new server to the configuration
    pub fn add_server(self: &mut Config, server: Server) {
        self.servers.push(server);
        self.flush_config();
    }
}
