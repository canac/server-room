use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
    pub project_name: String,
    pub start_command: String,
    run_times: Vec<u128>,
}

impl fmt::Display for Server {
    fn fmt(self: &Server, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.project_name)
    }
}

impl Server {
    // Create a new server
    pub fn new(project_name: String, start_command: String) -> Self {
        Server {
            project_name,
            start_command,
            run_times: vec![],
        }
    }

    // Calculate the likelihood that this server will be used again
    // Higher values are more likely, lower values are less likely
    pub fn get_weight(self: &Server) -> u128 {
        *self.run_times.last().unwrap_or(&0)
    }

    // Start up the server
    pub fn start(self: &mut Server) {
        // Record another run on this server
        self.run_times.push(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis(),
        );
    }
}