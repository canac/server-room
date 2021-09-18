use super::{Script, Server};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct RawServer {
    pub name: String,
    pub start_command: String,
    run_times: Vec<u128>,
}

// This struct represents the config structure in the raw JSON file
#[derive(Serialize, Deserialize)]
struct RawConfig {
    pub servers_dir: String,
    pub servers: Vec<RawServer>,
}

// This struct represents the config that is used by the rest of the application
#[derive(Clone)]
pub struct Config {
    pub servers_dir: String,
    pub servers: std::collections::HashMap<String, Server>,
}

impl Config {
    // Load the configuration from a string
    pub fn load_config() -> Config {
        // Load the configuration file contents
        let config_str = fs::read_to_string("config.json").expect("Error reading configuration");
        let raw_config: RawConfig =
            serde_json::from_str(&config_str).expect("Error parsing JSON string");
        let servers_dir = &raw_config.servers_dir;
        Config {
            servers_dir: servers_dir.clone(),
            // Build the servers map where servers are indexed by their project name from the servers vector
            servers: raw_config
                .servers
                .into_iter()
                .map(|server| {
                    (
                        server.name.clone(),
                        Server {
                            name: server.name.clone(),
                            project_dir: format!("{}/{}", servers_dir, server.name),
                            start_command: server.start_command,
                            run_times: server.run_times,
                        },
                    )
                })
                .collect(),
        }
    }

    // Write the configuration to disk
    pub fn flush_config(&self) {
        let mut raw_config = RawConfig {
            servers_dir: self.servers_dir.clone(),
            servers: self
                .servers
                .clone()
                .into_values()
                .map(|server| RawServer {
                    name: server.name,
                    start_command: server.start_command,
                    run_times: server.run_times,
                })
                .collect(),
        };
        // Sort the servers lexicographically by their name
        raw_config
            .servers
            .sort_by(|server1, server2| server1.name.cmp(&server2.name));
        fs::write(
            "config.json",
            serde_json::to_string_pretty(&raw_config).expect("Error stringifying config to JSON"),
        )
        .expect("Error writing configuration")
    }

    // Permanently add a new server to the configuration
    pub fn add_server(&self, project_name: String, start_command: String) {
        let mut new_config = self.clone();
        let project_dir = format!("{}/{}", self.servers_dir, project_name);
        new_config.servers.insert(
            project_name.clone(),
            Server::new(project_name, project_dir, start_command),
        );
        new_config.flush_config();
    }

    // Permanently set the start command of the specified server
    pub fn set_server_start_command(&self, server_name: &str, start_command: String) {
        let mut new_config = self.clone();
        new_config
            .servers
            .get_mut(server_name)
            .unwrap_or_else(|| panic!("Invalid server name {}", server_name))
            .start_command = start_command;
        new_config.flush_config();
    }

    // Permanently record a new start time
    pub fn record_server_run(&self, server_name: &str) {
        let mut new_config = self.clone();
        new_config
            .servers
            .get_mut(server_name)
            .unwrap_or_else(|| panic!("Invalid server name {}", server_name))
            .run_times
            .push(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis(),
            );
        new_config.flush_config();
    }

    // Permanently remove the server from the configuration
    pub fn remove_server(&self, server: &Server) {
        let mut new_config = self.clone();
        new_config.servers.remove(&server.name);
        new_config.flush_config();
    }

    // Determine whether the project name refers to a valid new project
    pub fn validate_new_project_name(&self, project_name: &str) -> Result<(), String> {
        match fs::metadata(format!(
            "{}/{}/package.json",
            self.servers_dir, project_name
        )) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err("Project package.json is not a file".to_string());
                }

                if self.servers.contains_key(project_name) {
                    return Err("Project already exists".to_string());
                }
            }
            Err(_) => return Err("Project doesn't have a valid package.json".to_string()),
        }

        Ok(())
    }

    // Return a vector of the project's start scripts
    pub fn load_project_start_scripts(&self, project_name: &str) -> Result<Vec<Script>, String> {
        let package_json_path = format!("{}/{}/package.json", self.servers_dir, project_name);
        let package_json_content = match fs::read_to_string(package_json_path) {
            Ok(content) => content,
            Err(_) => return Err("Error reading package.json".to_string()),
        };
        let package_json: Value = match serde_json::from_str(&package_json_content) {
            Ok(json) => json,
            Err(_) => return Err("Error parsing package.json".to_string()),
        };
        match &package_json["scripts"] {
            Value::Object(scripts) => Ok(scripts
                .iter()
                .map(|(name, command)| Script {
                    name: name.to_string(),
                    command: command.to_string(),
                })
                .collect::<Vec<_>>()),
            _ => Err("scripts property is not an object".to_string()),
        }
    }

    // Determine whether the start script for a project is valid
    pub fn validate_start_script(
        &self,
        project_name: &str,
        start_script: &str,
    ) -> Result<(), String> {
        let scripts = self.load_project_start_scripts(project_name)?;
        if !scripts.iter().any(|script| script.name == start_script) {
            return Err("Start script doesn't exist".to_string());
        }

        Ok(())
    }
}
