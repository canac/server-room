mod cli;
mod error;
mod project;
mod prompt;
mod script;
mod server;
mod server_store;

use cli::Cli;
use error::ApplicationError;
use project::Project;
use server_store::ServerStore;

use colored::*;
use directories::ProjectDirs;
use ngrammatic::CorpusBuilder;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

// Return the path to the server store file
fn get_store_path() -> Result<PathBuf, ApplicationError> {
    let project_dirs = ProjectDirs::from("com", "github.canac", "server-room")
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
            name,
            start_script,
            port,
        } => {
            let server_store = load_store()?;
            let absolute_path =
                fs::canonicalize(path.clone()).map_err(|_| ApplicationError::ParsePath(path))?;
            let mut project = Project::from_path(absolute_path)?;

            // Change the default name if one is provided
            if let Some(name) = name {
                project.name = name;
            }

            // Abort if the project is invalid before prompting the user for the start command
            server_store.validate_new_project(&project)?;

            let start_command = prompt::choose_start_command(
                &project,
                start_script,
                "Which npm script starts the server?",
            )?;
            let port = prompt::choose_port(port, "What port does the server listen on?")?;
            server_store.add_server(&project, start_command, port)
        }

        Cli::Edit(edit) => match edit {
            cli::Edit::Name {
                server,
                name,
                force,
            } => {
                let server_store = load_store()?;
                let server = prompt::choose_server(
                    &server_store,
                    server,
                    "Which server do you want to edit?",
                )?;
                let new_name =
                    prompt::choose_server_new_name(server, name, "What is the server's new name?")?;
                if prompt::confirm(force, "Are you sure you want to change the server's name?")? {
                    server_store.set_server_name(&server.name, new_name)?;
                }

                Ok(())
            }

            cli::Edit::StartScript {
                server,
                start_script,
                force,
            } => {
                let server_store = load_store()?;
                let server = prompt::choose_server(
                    &server_store,
                    server,
                    "Which server do you want to edit?",
                )?;
                let project = Project::from_path(server.get_project_dir())?;

                let new_start_script = prompt::choose_start_command(
                    &project,
                    start_script,
                    "Which npm script starts the server?",
                )?;

                if prompt::confirm(
                    force,
                    "Are you sure you want to change the server's start script?",
                )? {
                    server_store.set_server_start_command(&server.name, new_start_script)?;
                }

                Ok(())
            }

            cli::Edit::Port {
                server,
                port,
                force,
            } => {
                let server_store = load_store()?;
                let server = prompt::choose_server(
                    &server_store,
                    server,
                    "Which server do you want to edit?",
                )?;
                let new_port = prompt::choose_port(port, "What port does the server listen on?")?;
                if prompt::confirm(force, "Are you sure you want to change the server's port?")? {
                    server_store.set_server_port(&server.name, new_port)?;
                }

                Ok(())
            }
        },

        Cli::Run { server } => {
            let server_store = load_store()?;
            let server =
                prompt::choose_server(&server_store, server, "Which server do you want to run?")?;
            server_store.start_server(&server.name)
        }

        Cli::Remove { server, force } => {
            let server_store = load_store()?;
            let server = prompt::choose_server(
                &server_store,
                server,
                "Which server do you want to remove?",
            )?;
            if prompt::confirm(force, "Are you sure you want to remove the server?")? {
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
                ApplicationError::ParseStore(_) => Some("Make sure that the server store file contains valid TOML.".to_string()),
                ApplicationError::StringifyStore => None,
                ApplicationError::ReadPackageJson(project) => Some(format!("Try creating a new npm project in this project directory.\n\n    cd {:?}\n    npm init", project.dir)),
                ApplicationError::MalformedPackageJson { .. } => Some("Try making sure that your package.json contains valid JSON and that the \"scripts\" property is an object with at least one key. For example:\n\n    \"scripts\": {\n        \"start\": \"node app.js\"\n    }".to_string()),
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
                ApplicationError::EmptyServerName => Some(format!("Try providing a non-empty server name with `{}`", "--name".bold().cyan())),
                ApplicationError::DuplicateServerName(_) => Some(format!("Try giving the new server a unique name with `{}`", "--name".bold().cyan())),
                ApplicationError::DuplicateServerDir { existing, .. } => Some(format!(
                    "Try editing the existing server instead.\n\n    {}",
                    format!("server-room edit --server {}", existing.name).bold().cyan()
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
