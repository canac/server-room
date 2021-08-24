use dialoguer::{theme::ColorfulTheme, Select};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    servers_dir: String,
    servers: Vec<Server>,
}

// Load the configuration from a string
impl Config {
    fn load_config() -> Config {
        // Load the configuration file contents
        let config = fs::read_to_string("config.json").expect("Error reading configuration");
        serde_json::from_str(&config).expect("Error parsing JSON string")
    }

    // Write the configuration to disk
    fn flush_config(self: &Config) {
        fs::write(
            "config.json",
            serde_json::to_string_pretty(self).expect("Error stringifying config to JSON"),
        )
        .expect("Error writing configuration")
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Server {
    project_name: String,
    start_command: String,
    run_times: Vec<u128>,
}

impl fmt::Display for Server {
    fn fmt(self: &Server, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.project_name)
    }
}

impl Server {
    // Calculate the likelihood that this server will be used again
    // Higher values are more likely, lower values are less likely
    fn get_weight(self: &Server) -> u128 {
        *self.run_times.last().unwrap_or(&0)
    }
}

// Let the user pick a server from the defined list in the config
fn pick_server(config: &mut Config) -> &mut Server {
    // Sort the servers by most-recently-used first
    let mut mru_servers: Vec<&Server> = config.servers.iter().collect();
    mru_servers
        .sort_by(|server1, server2| server1.get_weight().cmp(&server2.get_weight()).reverse());

    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a server")
        .default(0)
        .items(&mru_servers)
        .interact()
        .unwrap();

    // Convert the index in the sorted servers vector into an index in the original servers vector
    let index = config
        .servers
        .iter()
        .position(|server| server.project_name == mru_servers[selected].project_name)
        .unwrap();
    &mut config.servers[index]
}

fn main() {
    let mut config = Config::load_config();
    let server = pick_server(&mut config);

    // Record another run on this server
    server.run_times.push(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis(),
    );

    config.flush_config();
}
