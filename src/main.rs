use dialoguer::{theme::ColorfulTheme, Select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    servers_dir: String,
    servers: Vec<Server>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Server {
    project_name: String,
    start_command: String,
    run_times: Vec<u128>,
}

// Load the configuration from a string
fn load_config() -> Config {
    // Load the configuration file contents
    let config = fs::read_to_string("config.json").expect("Error reading configuration");
    serde_json::from_str(&config).expect("Error parsing JSON string")
}

// Write the configuration to disk
fn flush_config(config: &Config) {
    fs::write(
        "config.json",
        serde_json::to_string(config).expect("Error stringifying config to JSON"),
    )
    .expect("Error writing configuration")
}

// Let the user pick a server from the defined list in the config
fn pick_server(config: &mut Config) -> &mut Server {
    let server_options: Vec<&String> = config
        .servers
        .iter()
        .map(|server| &server.project_name)
        .collect();

    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a server")
        .default(0)
        .items(&server_options)
        .interact()
        .unwrap();

    &mut config.servers[selected]
}

fn main() {
    let mut config = load_config();
    let server = pick_server(&mut config);

    // Record another run on this server
    server.run_times.push(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis(),
    );

    flush_config(&config);
}
