use super::actionable_error::{ActionableError, ErrorCode};
use super::{Script, Server};
use ngrammatic::CorpusBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::f64::consts::LN_2;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct RawServer {
    pub name: String,
    pub start_command: String,
    frecency: f64,
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
                            frecency: server.frecency,
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
                    frecency: server.frecency,
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
        let server = new_config
            .servers
            .get_mut(server_name)
            .unwrap_or_else(|| panic!("Invalid server name {}", server_name));

        // Uses the frecency algorithm described here https://wiki.mozilla.org/User:Jesse/NewFrecency
        const FRECENCY_HALF_LIFE_MICROS: f64 = 30f64 * 24f64 * 60f64 * 60f64 * 1_000_000f64; // one month
        const DECAY: f64 = LN_2 / FRECENCY_HALF_LIFE_MICROS as f64;
        const SCORE_INCREASE_PER_RUN: f64 = 1f64;
        let now_decay = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros() as f64
            * DECAY;
        let score = (server.frecency - now_decay).exp();
        let new_score = score + SCORE_INCREASE_PER_RUN;
        server.frecency = new_score.ln() + now_decay;
        new_config.flush_config();
    }

    // Permanently remove the server from the configuration
    pub fn remove_server(&self, server: &Server) {
        let mut new_config = self.clone();
        new_config.servers.remove(&server.name);
        new_config.flush_config();
    }

    // Determine whether the project name refers to a valid new project
    pub fn validate_new_project_name(&self, project_name: &str) -> Result<(), ActionableError> {
        let package_json_path = format!("{}/{}/package.json", self.servers_dir, project_name);
        let metadata = fs::metadata(format!(
            "{}/{}/package.json",
            self.servers_dir, project_name
        )).map_err(|_| {
            ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try creating a new npm project in this project directory.\n\n    cd {}/{}\n    npm init", self.servers_dir, project_name),
            }
        })?;

        if !metadata.is_file() {
            return Err(ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try making sure that {} is a file.", package_json_path),
            });
        }

        if self.servers.contains_key(project_name) {
            return Err(ActionableError {
                code: ErrorCode::DuplicateProject,
                message: format!("Project {} already exists", project_name),
                suggestion: format!(
                    "Try editing the existing project instead.\n\n    server-room edit --server {}",
                    project_name
                ),
            });
        }

        Ok(())
    }

    // Return a vector of the project's start scripts
    pub fn load_project_start_scripts(
        &self,
        project_name: &str,
    ) -> Result<Vec<Script>, ActionableError> {
        let package_json_path = format!("{}/{}/package.json", self.servers_dir, project_name);
        let package_json_content = fs::read_to_string(&package_json_path).map_err(|_| {
            ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try creating a new npm project in this project directory.\n\n    cd {}/{}\n    npm init", self.servers_dir, project_name),
            }
        })?;
        let package_json: Value =
            serde_json::from_str(&package_json_content).map_err(|_| ActionableError {
                code: ErrorCode::ParsePackageJson,
                message: format!("Could not parse {}", package_json_path),
                suggestion: "Try making sure that package.json contains valid JSON.".to_string(),
            })?;
        let scripts = package_json["scripts"].as_object().and_then(|scripts| {
            if scripts.is_empty() { None } else { Some(scripts) }
        }).ok_or_else(|| ActionableError {
                code: ErrorCode::ParsePackageJson,
                message: format!("Property \"scripts\" in {} is not an object or is empty", package_json_path),
                suggestion: "Try making sure that the \"scripts\" property in package.json is an object with at least one key. For example:\n\n    \"scripts\": {\n        \"start\": \"node app.js\"\n    }".to_string(),
            })?;
        Ok(scripts
            .iter()
            .map(|(name, command)| Script {
                name: name.to_string(),
                command: command.to_string(),
            })
            .collect::<Vec<_>>())
    }

    // Determine whether the start script for a project is valid
    pub fn validate_start_script(
        &self,
        project_name: &str,
        start_script: &str,
    ) -> Result<(), ActionableError> {
        let scripts = self.load_project_start_scripts(project_name)?;
        if !scripts.iter().any(|script| script.name == start_script) {
            let package_json_path = format!("{}/{}/package.json", self.servers_dir, project_name);
            return Err(ActionableError {
                code: ErrorCode::MissingStartScript,
                message: format!("No script {} in {}", start_script, package_json_path),
                suggestion: format!(
                    "Try adding the script {} to your package.json.",
                    start_script
                ),
            });
        }

        Ok(())
    }

    // Return the name of the server closest to the provided server name
    pub fn get_closest_server_name(&self, server_name: &str) -> Option<String> {
        let mut corpus = CorpusBuilder::new().finish();
        for server_name in self.servers.keys() {
            corpus.add_text(server_name);
        }
        let results = corpus.search(server_name, 0f32);
        results.first().map(|result| result.text.clone())
    }
}
