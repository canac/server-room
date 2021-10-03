use super::config::Config;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

#[derive(Clone, Deserialize, Serialize)]
pub struct Server {
    pub name: String,
    pub start_command: String,
    pub frecency: f64,

    #[serde(skip)]
    config: Option<Rc<Config>>,
}

impl fmt::Display for Server {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.name)
    }
}

impl Server {
    // Create a new server
    pub fn new(name: String, start_command: String) -> Self {
        Server {
            name,
            start_command,
            frecency: 0f64,
            config: None,
        }
    }

    // Link the server to a global config
    pub fn link(&mut self, config: Rc<Config>) {
        self.config = Some(config);
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
    // Requires the the config be set first
    pub fn get_project_dir(&self) -> PathBuf {
        self.config
            .as_ref()
            .unwrap()
            .get_servers_dir()
            .join(&self.name)
    }
}
