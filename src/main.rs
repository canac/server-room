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

fn main() {
    println!("{:?}", load_config());
}
