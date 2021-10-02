use super::actionable_error::{ActionableError, ErrorCode};
use super::{Config, Script, Server};
use ngrammatic::CorpusBuilder;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::f64::consts::LN_2;
use std::fs;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

// This struct represents the user-configured servers used by the rest of the application
// It is stored as a vector in the Datastore, but is deserialized into a hashmap of servers, where
// the key is the server name
#[derive(Clone)]
pub struct ServerStore {
    servers: std::collections::HashMap<String, Server>,
    config: Option<Rc<Config>>,
}

#[derive(Deserialize, Serialize)]
pub struct RawServerStore {
    servers: Vec<Server>,
}

// Implement a custom deserializer that deserializes the raw servers vector into a hash map
impl<'de> Deserialize<'de> for ServerStore {
    fn deserialize<D>(deserializer: D) -> Result<ServerStore, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ServerStore {
            servers: RawServerStore::deserialize(deserializer)?
                .servers
                .into_iter()
                .map(|server| (server.name.clone(), server))
                .collect(),
            config: None,
        })
    }
}

// Implement a custom serializer that serializes the servers hash map into a vector
impl Serialize for ServerStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut servers = self.servers.clone().into_values().collect::<Vec<_>>();

        // Sort the servers lexicographically by their name
        servers.sort_by(|server1, server2| server1.name.cmp(&server2.name));

        let raw = RawServerStore { servers };
        RawServerStore::serialize(&raw, serializer)
    }
}

impl ServerStore {
    // Load the data store from disk
    pub fn load(config: Rc<Config>) -> ServerStore {
        let server_store_str =
            fs::read_to_string("servers.json").expect("Error reading server store");
        let mut store: ServerStore =
            serde_json::from_str(&server_store_str).expect("Error parsing JSON string");
        store.link(config);
        store
    }

    // Write the data store to disk
    pub fn flush(&self) {
        fs::write(
            "servers.json",
            serde_json::to_string_pretty(&self).expect("Error stringifying config to JSON"),
        )
        .expect("Error writing server store")
    }

    // Link the server store to a global config
    pub fn link(&mut self, config: Rc<Config>) {
        self.config = Some(config.clone());
        self.servers
            .values_mut()
            .for_each(|server| server.link(config.clone()))
    }

    // Permanently add a new server to the configuration
    pub fn add_server(&self, project_name: String, start_command: String) {
        let mut new_store = self.clone();
        let mut server = Server::new(project_name.clone(), start_command);
        server.link(self.config.as_ref().unwrap().clone());
        new_store.servers.insert(project_name, server);
        new_store.flush();
    }

    // Permanently set the start command of the specified server
    pub fn set_server_start_command(&self, server_name: &str, start_command: String) {
        let mut new_store = self.clone();
        new_store
            .servers
            .get_mut(server_name)
            .unwrap_or_else(|| panic!("Invalid server name {}", server_name))
            .start_command = start_command;
        new_store.flush();
    }

    // Permanently record a new start time
    pub fn start_server(&self, server_name: &str) {
        let mut new_store = self.clone();

        let server = new_store
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
        new_store.flush();

        new_store.servers.get(server_name).unwrap().start();
    }

    // Permanently remove the server from the store
    pub fn remove_server(&self, server: &Server) {
        let mut new_store = self.clone();
        new_store.servers.remove(&server.name);
        new_store.flush();
    }

    // Determine whether the project name refers to a valid new project
    pub fn validate_new_project_name(&self, project_name: &str) -> Result<(), ActionableError> {
        let servers_dir = self.config.as_ref().unwrap().get_servers_dir();
        let package_json_path = format!("{}/{}/package.json", &servers_dir, project_name);
        let metadata = fs::metadata(format!(
            "{}/{}/package.json",
            servers_dir, project_name
        )).map_err(|_| {
            ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try creating a new npm project in this project directory.\n\n    cd {}/{}\n    npm init", self.config.as_ref().unwrap().get_servers_dir(), project_name),
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
        let package_json_path = format!(
            "{}/{}/package.json",
            self.config.as_ref().unwrap().get_servers_dir(),
            project_name
        );
        let package_json_content = fs::read_to_string(&package_json_path).map_err(|_| {
            ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try creating a new npm project in this project directory.\n\n    cd {}/{}\n    npm init", self.config.as_ref().unwrap().get_servers_dir(), project_name),
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
        let servers_dir = self.config.as_ref().unwrap().get_servers_dir();
        if !scripts.iter().any(|script| script.name == start_script) {
            let package_json_path = format!("{}/{}/package.json", servers_dir, project_name);
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

    pub fn get_one(&self, server_name: String) -> Option<&Server> {
        self.servers.get(&server_name)
    }

    pub fn get_all(&self) -> Vec<&Server> {
        self.servers.values().collect::<Vec<_>>()
    }
}
