use super::Config;
use std::fmt;
use std::process::Command;

#[derive(Clone)]
pub struct Server {
    pub name: String,
    pub project_dir: String,
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
    pub fn new(name: String, project_dir: String, start_command: String) -> Self {
        Server {
            name,
            project_dir,
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
    pub fn start(&self, config: &Config) {
        // Record another run on this server
        config.record_server_run(&self.name);

        // Execute the server's start command, sending input and output to stdin and stdout
        Command::new("sh")
            .args(["-c", self.start_command.as_str()])
            .current_dir(self.project_dir.as_str())
            .status()
            .unwrap_or_else(|_| {
                panic!("Failed to execute \"{}\" start command", self.start_command)
            });
    }
}
