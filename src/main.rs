use dialoguer::{theme::ColorfulTheme, Select};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    servers_dir: String,
    servers: Vec<Server>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Server {
    project_name: String,
    start_command: String,
}

// Load the configuration from a string
fn load_config() -> Config {
    // Load the configuration file contents
    let config = fs::read_to_string("config.json").expect("Error reading configuration");
    serde_json::from_str(&config).expect("Error parsing JSON string")
}

// Let the user pick a server from the defined list in the config
fn pick_server(config: &Config) -> &Server {
    let server_options: Vec<&String> = config
        .servers
        .iter()
        .map(|server| &server.project_name)
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a server")
        .default(0)
        .items(&server_options)
        .interact()
        .unwrap();

    &config.servers[selection]
}

fn main() {
    let config = load_config();
    let server = pick_server(&config);
    println!("{:?}", server);
}
