use super::error::ApplicationError;
use super::project::Project;
use super::server::Server;
use super::server_store::ServerStore;

use inquire::{Confirm, CustomType, Select, Text};
use std::collections::HashSet;
use std::iter::FromIterator;

// Get an existing server from the command line argument, falling back to letting the user interactively pick one
pub fn choose_server<'s>(
    server_store: &'s ServerStore,
    cli_server_name: Option<String>,
    prompt: &str,
) -> Result<&'s Server, ApplicationError> {
    match cli_server_name {
        Some(server_name) => server_store.get_one(server_name.as_str()),
        None => {
            // If no server was provided, let the user pick one
            let mut servers = server_store.get_all();
            // Put the servers with the highest weight first
            servers.sort_by(|server1, server2| {
                server1
                    .get_weight()
                    .partial_cmp(&server2.get_weight())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .reverse()
                    .then_with(|| server1.name.cmp(&server2.name))
            });

            if servers.is_empty() {
                return Err(ApplicationError::NoServers);
            }

            Select::new(prompt, servers)
                .prompt()
                .map_err(ApplicationError::from)
        }
    }
}

// Get the start command from the script name command line argument, falling back to letting the user interactively pick one
pub fn choose_start_command(
    project: &Project,
    cli_start_script: Option<String>,
    prompt: &str,
) -> Result<String, ApplicationError> {
    let start_script = match cli_start_script {
        Some(start_script) => {
            // If a start script name was provided from the command line, validate it
            project.get_start_script(start_script)?
        }
        None => {
            // If no start script was provided, let the user pick one
            let mut scripts = project.get_start_scripts()?;
            let priority_scripts = HashSet::<&&str>::from_iter(["dev", "run", "start"].iter());
            // Sort the scripts by name, but put priority scripts first
            scripts.sort_by(|script1, script2| {
                priority_scripts
                    .contains(&script1.name.as_str())
                    .cmp(&priority_scripts.contains(&script2.name.as_str()))
                    .reverse()
                    .then_with(|| script1.name.cmp(&script2.name))
            });
            Select::new(prompt, scripts).prompt()?
        }
    };
    Ok(format!("npm run {}", start_script.name))
}

// Get the new name for an existing server from the command line argument, falling back to letting the user choose one
pub fn choose_server_new_name(
    server: &Server,
    cli_new_name: Option<String>,
    prompt: &str,
) -> Result<String, ApplicationError> {
    match cli_new_name {
        Some(new_name) => Ok(new_name),
        None => Text::new(prompt)
            .with_placeholder(server.name.as_str())
            .prompt()
            .map_err(ApplicationError::InquireError),
    }
}

// Get the port for a server from the command line argument, falling back to letting the user choose one
pub fn choose_port(cli_port: Option<u16>, prompt: &str) -> Result<u16, ApplicationError> {
    match cli_port {
        Some(port) => Ok(port),
        None => CustomType::<u16>::new(prompt)
            .with_error_message("Please enter a valid port number")
            .prompt()
            .map_err(ApplicationError::InquireError),
    }
}

// Get confirmation to perform the operation from command line argument, falling back to prompting the user for confirmation
pub fn confirm(cli_confirm: bool, prompt: &str) -> Result<bool, ApplicationError> {
    if cli_confirm {
        Ok(true)
    } else {
        Ok(Confirm::new(prompt)
            .with_default(false)
            .prompt_skippable()?
            .unwrap_or(false))
    }
}
