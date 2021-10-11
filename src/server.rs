use super::error::ApplicationError;
use super::project::Project;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

// This struct represents the server as used by the rest of the application
#[derive(Clone, Deserialize, Serialize)]
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
    pub fn new(name: String, dir: PathBuf, start_command: String) -> Self {
        Server {
            name,
            dir,
            start_command,
            frecency: 0f64,
        }
    }

    // Create a new server from a project
    pub fn from_project(project: Project, start_command: String) -> Self {
        Self::new(project.name, project.dir, start_command)
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
