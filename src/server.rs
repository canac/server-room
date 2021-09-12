use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
    pub project_name: String,
    pub start_command: String,
    pub run_times: Vec<u128>,
}

impl fmt::Display for Server {
    fn fmt(self: &Server, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.project_name)
    }
}

impl Server {
    // Calculate the likelihood that this server will be used again
    // Higher values are more likely, lower values are less likely
    pub fn get_weight(self: &Server) -> u128 {
        *self.run_times.last().unwrap_or(&0)
    }
}
