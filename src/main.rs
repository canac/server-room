mod cli;
mod error;
mod project;
mod script;
mod server;
mod server_store;

use cli::Cli;
use error::ApplicationError;
use project::Project;
use server::Server;
use server_store::ServerStore;

use colored::*;
use directories::ProjectDirs;
use inquire::{Confirm, Select, Text};
use ngrammatic::CorpusBuilder;
use std::collections::HashSet;
use std::fs;
use std::iter::FromIterator;
use std::path::PathBuf;
use structopt::StructOpt;

// Get an existing server from the command line argument, falling back to letting the user interactively pick one
fn get_existing_server_from_user<'s>(
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

// Get an alias for the project if necessary from the command line argument, falling back to letting the user pick one
// An alias is only necessary if the project has the name name as an existing project
fn get_alias_from_user(
    server_store: &ServerStore,
    project: &Project,
    cli_alias: Option<String>,
) -> Result<String, ApplicationError> {
    // Get an alias from the user if a server with this name already exists
    let mut alias = cli_alias.unwrap_or_else(|| project.name.clone());
    loop {
        // Loop until an unused server name is provided
        if server_store.get_one(alias.as_str()).is_ok() {
            alias = Text::new("Choose an alias")
                .with_help_message(
                    "A server with this name already exists, so the new server must be aliased.",
                )
                .prompt()
                .map_err(ApplicationError::from)?;
        } else {
            break;
        }
    }

    Ok(alias)
}

// Get the start command from the script name command line argument, falling back to letting the user interactively pick one
fn get_start_command_from_user(
    project: &Project,
    cli_start_script: Option<String>,
    prompt: &str,
) -> Result<String, ApplicationError> {
    let start_script = match cli_start_script {
        Some(start_script) => {
            // If a start script name was provided from the command line, validate it
            project.validate_start_script(start_script.as_str())?;
            start_script
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
            let start_script = Select::new(prompt, scripts).prompt()?;
            start_script.name
        }
    };
    Ok(format!("npm run {}", start_script))
}

// Get confirmation to perform the operation from command line argument, falling back to prompting the user for confirmation
fn get_confirmation_from_user(cli_confirm: bool, prompt: &str) -> Result<bool, ApplicationError> {
    if cli_confirm {
        Ok(true)
    } else {
        Ok(Confirm::new(prompt)
            .with_default(false)
            .prompt_skippable()?
            .unwrap_or(false))
    }
}

// Return the path to the server store file
fn get_store_path() -> Result<PathBuf, ApplicationError> {
    let project_dirs = ProjectDirs::from("com", "Canac Apps", "Server Room")
        .ok_or(ApplicationError::ProjectDirs)?;
    Ok(project_dirs.data_dir().join(PathBuf::from("servers.toml")))
}

fn run() -> Result<(), ApplicationError> {
    let cli = Cli::from_args();
    match cli {
        Cli::Config => {
            println!("Server store path: {:?}", get_store_path()?);
            Ok(())
        }

        Cli::Add {
            path,
            alias,
            start_script,
        } => {
            let server_store = load_store()?;
            let absolute_path =
                fs::canonicalize(path.clone()).map_err(|_| ApplicationError::ParsePath(path))?;

            let mut project = Project::from_path(absolute_path)?;
            project.name = get_alias_from_user(&server_store, &project, alias)?;

            let start_command =
                get_start_command_from_user(&project, start_script, "Pick a start script")?;
            server_store.add_server(&project, start_command)
        }

        Cli::Edit {
            server,
            start_script,
            force,
        } => {
            let server_store = load_store()?;
            let server =
                get_existing_server_from_user(&server_store, server, "Pick a server to edit")?;
            let project = Project::from_path(server.get_project_dir())?;
            let start_command =
                get_start_command_from_user(&project, start_script, "Pick a new start script")?;
            if get_confirmation_from_user(
                force,
                "Are you sure you want to change the server's start script?",
            )? {
                server_store.set_server_start_command(&server.name, start_command)
            } else {
                Ok(())
            }
        }

        Cli::Run { server } => {
            let server_store = load_store()?;
            let server =
                get_existing_server_from_user(&server_store, server, "Pick a server to run")?;
            server_store.start_server(&server.name)
        }

        Cli::Remove { server, force } => {
            let server_store = load_store()?;
            let server =
                get_existing_server_from_user(&server_store, server, "Pick a server to remove")?;
            if get_confirmation_from_user(force, "Are you sure you want to remove the server?")? {
                server_store.remove_server(&server.name)
            } else {
                Ok(())
            }
        }

        Cli::List => {
            let server_store = load_store()?;
            println!("{}", "Servers:".bold());
            server_store.get_all().iter().for_each(|server| {
                println!(
                    "{} ({})",
                    server.name.bold().green(),
                    server.start_command.cyan()
                )
            });
            Ok(())
        }

        Cli::Unknown(args) => Err(ApplicationError::InvalidCommand(args[0].clone())),
    }
}

// Load the server store
fn load_store() -> Result<ServerStore, ApplicationError> {
    ServerStore::load(get_store_path()?)
}

fn main() {
    let exit_code = match run() {
        Ok(_) => 0,
        Err(err) => {
            // Generate user-facing suggestions based on the error
            let suggestion: Option<String> = match &err {
                ApplicationError::ProjectDirs => None,
                ApplicationError::WriteStore(_) => Some("Make sure that the server store file is writable.".to_string()),
                ApplicationError::ParseStore(_) => Some("Make sure that the server store file is valid TOML.".to_string()),
                ApplicationError::StringifyStore => None,
                ApplicationError::ReadPackageJson(servers_dir) => Some(format!("Try creating a new npm project in this project directory.\n\n    cd {:?}\n    npm init", servers_dir)),
                ApplicationError::MalformedPackageJson { path: _, cause: _ } => Some("Try making sure that your package.json contains valid JSON and that the \"scripts\" property is an object with at least one key. For example:\n\n    \"scripts\": {\n        \"start\": \"node app.js\"\n    }".to_string()),
                ApplicationError::ParsePath(_) => None,
                ApplicationError::NonExistentScript {
                    project,
                    script,
                } => {
                    let mut corpus = CorpusBuilder::new().finish();
                    project.get_start_scripts().unwrap_or_else(|_| vec![]).iter().for_each(|script| {
                        corpus.add_text(script.name.as_str())
                    });
                    let results = corpus.search(script, 0f32);
                    let suggestion = results.first().map(|result| result.text.clone());
                    Some(match suggestion {
                        Some(suggestion) => format!("Did you mean `{}`?", format!("--start-script {}", suggestion).bold().cyan()),
                        None => format!("Try adding the script {} to your package.json.", script)
                    })
                },
                ApplicationError::RunScript(_) => Some("Make sure that the command is spelled correctly and is in the path.".to_string()),
                ApplicationError::NonExistentServer(server) => {
                    let suggested_server = load_store().ok().and_then(|server_store| {
                        server_store.get_closest_server_name(server)
                    });
                    Some(match suggested_server {
                        Some(suggestion) => format!("Did you mean `{}`?", format!("--server {}", suggestion).bold().cyan()),
                        None => "Try a different server name.".to_string(),
                    })
                },
                ApplicationError::DuplicateServer(server) => Some(format!(
                    "Try editing the existing server instead.\n\n    {}",
                    format!("server-room edit --server {}", server).bold().cyan()
                )),
                ApplicationError::NoServers => Some("Try adding a new server first.\n\n    server-room add".to_string()),
                ApplicationError::InquireError(_) => None,
                ApplicationError::InvalidCommand(command) => {
                    let mut corpus = CorpusBuilder::new().finish();
                    corpus.add_text("config");
                    corpus.add_text("add");
                    corpus.add_text("edit");
                    corpus.add_text("run");
                    corpus.add_text("remove");
                    corpus.add_text("rm");
                    corpus.add_text("list");
                    corpus.add_text("ls");
                    let results = corpus.search(command.as_str(), 0.5f32);
                    Some(match results.first() {
                        Some(result) => format!("Did you mean `{}`?", format!("server-room {}", result.text).bold().cyan()),
                        None => "Try `server-room --help` to see available subcommands.".to_string(),
                    })
                }
            };

            eprintln!("{}: {}", "Error".bold().red(), err);
            if let Some(suggestion) = suggestion {
                eprintln!("{}", suggestion);
            }
            1
        }
    };

    std::process::exit(exit_code);
}
