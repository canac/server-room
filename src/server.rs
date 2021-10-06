use super::config::Config;
use super::error::ApplicationError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

// This struct represents the server as stored in the server store
#[derive(Deserialize, Serialize)]
pub struct RawServer {
    pub name: String,
    pub start_command: String,
    pub frecency: f64,
}
// This struct represents the server as used by the rest of the application
#[derive(Clone)]
pub struct Server {
    pub name: String,
    pub dir: PathBuf,
    pub start_command: String,
    pub frecency: f64,
}

impl fmt::Display for Server {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.name)
    }
}

impl Server {
    // Create a new server
    pub fn new(config: &Config, name: String, start_command: String) -> Self {
        Server {
            name: name.clone(),
            dir: config.get_servers_dir().join(name),
            start_command,
            frecency: 0f64,
        }
    }

    // Create a new server from a raw server
    pub fn from_raw(config: &Config, raw: RawServer) -> Self {
        Server {
            name: raw.name.clone(),
            dir: config.get_servers_dir().join(raw.name),
            start_command: raw.start_command,
            frecency: raw.frecency,
        }
    }

    // Create a new server from a raw server
    pub fn into_raw(self) -> RawServer {
        RawServer {
            name: self.name,
            start_command: self.start_command,
            frecency: self.frecency,
        }
    }

    // Calculate the likelihood that this server will be used again
    // Higher values are more likely, lower values are less likely
    pub fn get_weight(&self) -> f64 {
        self.frecency
    }

    // Start up the server
    pub fn start(&self) -> Result<(), ApplicationError> {
        // Execute the server's start command, sending input and output to stdin and stdout
        let status = Command::new("sh")
            .args(["-c", self.start_command.as_str()])
            .current_dir(self.get_project_dir())
            .status();
        match status {
            Ok(_) => Ok(()),
            Err(_) => Err(ApplicationError::RunScript(self.start_command.clone())),
        }
    }

    // Calculate the server's project dir
    pub fn get_project_dir(&self) -> PathBuf {
        self.dir.clone()
    }
}
