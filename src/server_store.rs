use super::error::ApplicationError;
use super::project::Project;
use super::{Config, Server};
use ngrammatic::CorpusBuilder;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::f64::consts::LN_2;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

// This struct represents the user-configured servers used by the rest of the application
// It is stored as a vector in the Datastore, but is deserialized into a hashmap of servers, where
// the key is the server name
#[derive(Clone)]
pub struct ServerStore {
    servers: std::collections::HashMap<String, Server>,
    config: Rc<Config>,
}

#[derive(Deserialize, Serialize)]
pub struct RawServerStore {
    servers: Vec<Server>,
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
    pub fn load(config: Rc<Config>) -> Result<ServerStore, ApplicationError> {
        let store_path = PathBuf::from("servers.json");
        let server_store_str = fs::read_to_string(&store_path)
            .map_err(|_| ApplicationError::ReadStore(store_path.clone()))?;
        let raw_store: RawServerStore = serde_json::from_str(&server_store_str)
            .map_err(|_| ApplicationError::ParseStore(store_path))?;
        Ok(ServerStore {
            servers: raw_store
                .servers
                .into_iter()
                .map(|server| (server.name.clone(), server))
                .collect(),
            config,
        })
    }

    // Write the data store to disk
    pub fn flush(&self) -> Result<(), ApplicationError> {
        let store_path = PathBuf::from("servers.json");
        let stringified = serde_json::to_string_pretty(&self)
            .map_err(|_| ApplicationError::StringifyStore(store_path.clone()))?;
        fs::write(&store_path, stringified)
            .map_err(|_| ApplicationError::WriteStore(store_path))?;
        Ok(())
    }

    // Permanently add a new server to the server store
    pub fn add_server(
        &self,
        project: &Project,
        start_command: String,
    ) -> Result<(), ApplicationError> {
        // Make sure the project doesn't already exist
        if self.servers.contains_key(&project.name) {
            return Err(ApplicationError::DuplicateServer(project.name.clone()));
        }

        let mut new_store = self.clone();
        let server = Server::new(self.config.as_ref(), project.name.clone(), start_command);
        new_store.servers.insert(project.name.clone(), server);
        new_store.flush()
    }

    // Permanently set the start command of the specified server
    pub fn set_server_start_command(
        &self,
        server_name: &str,
        start_command: String,
    ) -> Result<(), ApplicationError> {
        let mut new_store = self.clone();
        let server = new_store.get_one_mut(server_name)?;
        server.start_command = start_command;
        new_store.flush()
    }

    // Permanently record a new start time
    pub fn start_server(&self, server_name: &str) -> Result<(), ApplicationError> {
        let mut new_store = self.clone();
        let mut server = new_store.get_one_mut(server_name)?;

        // Uses the frecency algorithm described here https://wiki.mozilla.org/User:Jesse/NewFrecency
        const FRECENCY_HALF_LIFE_MICROS: f64 = 30f64 * 24f64 * 60f64 * 60f64 * 1_000_000f64; // one month
        const DECAY: f64 = LN_2 / FRECENCY_HALF_LIFE_MICROS as f64;
        const SCORE_INCREASE_PER_RUN: f64 = 1f64;
        let now_decay = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_micros(0))
            .as_micros() as f64
            * DECAY;
        let score = (server.frecency - now_decay).exp();
        let new_score = score + SCORE_INCREASE_PER_RUN;
        server.frecency = new_score.ln() + now_decay;
        new_store.flush()?;

        new_store.get_one(server_name)?.start()
    }

    // Permanently remove the server from the store
    pub fn remove_server(&self, server: &Server) -> Result<(), ApplicationError> {
        let mut new_store = self.clone();
        new_store.servers.remove(&server.name);
        new_store.flush()
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

    pub fn get_one(&self, server_name: &str) -> Result<&Server, ApplicationError> {
        self.servers
            .get(server_name)
            .ok_or_else(|| ApplicationError::NonExistentServer(server_name.to_string()))
    }

    pub fn get_one_mut(&mut self, server_name: &str) -> Result<&mut Server, ApplicationError> {
        self.servers
            .get_mut(server_name)
            .ok_or_else(|| ApplicationError::NonExistentServer(server_name.to_string()))
    }

    pub fn get_all(&self) -> Vec<&Server> {
        self.servers.values().collect::<Vec<_>>()
    }
}
