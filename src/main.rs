use clap::{App, Arg, SubCommand};
use inquire::Select;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

struct Script {
    name: String,
    command: String,
}

impl fmt::Display for Script {
    fn fmt(self: &Script, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}: {}", self.name, self.command)
    }
}

// Let the user pick a server from the defined list in the config
fn pick_server(config: &mut Config) -> &mut Server {
    let mut mru_servers: Vec<&mut Server> = config.servers.iter_mut().collect();
    mru_servers
        .sort_by(|server1, server2| server1.get_weight().cmp(&server2.get_weight()).reverse());
    Select::new("Pick a server", mru_servers).prompt().unwrap()
}

// Add a new server to the config
fn add_server(config: &mut Config, project_name: &str) {
    let package_json_path = format!("{}/{}/package.json", config.servers_dir, project_name);
    let package_json_content =
        fs::read_to_string(package_json_path).expect("Error reading package.json");
    let package_json: Value = serde_json::from_str(&package_json_content).unwrap();
    if let Value::Object(scripts_json) = &package_json["scripts"] {
        let scripts = scripts_json
            .iter()
            .map(|(name, command)| Script {
                name: name.to_string(),
                command: command.to_string(),
            })
            .collect::<Vec<_>>();
        let script = Select::new("Pick a start command", scripts)
            .prompt()
            .unwrap();
        config.servers.push(Server {
            project_name: project_name.to_string(),
            start_command: format!("npm run {}", script.name),
            run_times: vec![],
        })
    } else {
        panic!("scripts property is not an object");
    }
}

fn main() {
    let mut config = Config::load_config();

    let matches = App::new("server-room")
        .version("0.1.0")
        .author("Caleb Cox")
        .about("Runs dev servers")
        .subcommand(
            SubCommand::with_name("add")
                .about("add a new server")
                .arg(
                    Arg::with_name("project-name")
                        .help("Identifies the project name")
                        .takes_value(true)
                        .short("p")
                        .long("project-name")
                        .required(true),
                )
                .arg(
                    Arg::with_name("start-script")
                        .help("Sets the new server's start script")
                        .takes_value(true)
                        .short("s")
                        .long("start-script"),
                ),
        )
        .subcommand(SubCommand::with_name("run").about("run a server"))
        .get_matches();

    match matches.subcommand_name() {
        Some("add") => {
            add_server(
                &mut config,
                matches
                    .subcommand_matches("add")
                    .unwrap()
                    .value_of("project-name")
                    .unwrap(),
            );
            config.flush_config();
        }
        Some("run") => {
            // Record another run on this server
            let server = pick_server(&mut config);
            server.run_times.push(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis(),
            );
            config.flush_config();
        }
        _ => println!("Some other subcommand was used"),
    }
}
