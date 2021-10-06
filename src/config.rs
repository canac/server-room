use super::error::ApplicationError;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

// This struct represents the config that is used by the rest of the application
#[derive(Deserialize)]
pub struct Config {
    // The directory where all of the servers are located
    servers_dir: PathBuf,
}

impl Config {
    // Read the configuration from disk
    pub fn load(config_path: PathBuf) -> Result<Config, ApplicationError> {
        let config_str = fs::read_to_string(&config_path)
            .map_err(|_| ApplicationError::ReadConfig(config_path.clone()))?;
        toml::from_str(&config_str).map_err(|_| ApplicationError::ParseConfig(config_path))
    }

    // Return the config's servers_dir
    pub fn get_servers_dir(&self) -> PathBuf {
        self.servers_dir.clone()
    }
}
