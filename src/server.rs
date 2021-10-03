use super::config::Config;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Deserialize, Serialize)]
pub struct Server {
    pub name: String,
    #[serde(skip)]
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

    // Calculate the likelihood that this server will be used again
    // Higher values are more likely, lower values are less likely
    pub fn get_weight(&self) -> f64 {
        self.frecency
    }

    // Start up the server
    pub fn start(&self) {
        // Execute the server's start command, sending input and output to stdin and stdout
        Command::new("sh")
            .args(["-c", self.start_command.as_str()])
            .current_dir(self.get_project_dir())
            .status()
            .unwrap_or_else(|_| {
                panic!("Failed to execute \"{}\" start command", self.start_command)
            });
    }

    // Calculate the server's project dir
    pub fn get_project_dir(&self) -> PathBuf {
        self.dir.clone()
    }
}
