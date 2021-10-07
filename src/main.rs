mod config;
mod error;
mod project;
mod script;
mod server;
mod server_store;

use config::Config;
use error::ApplicationError;
use project::Project;
use server::Server;
use server_store::ServerStore;

use clap::{App, AppSettings, Arg, SubCommand};
use colored::*;
use directories::ProjectDirs;
use inquire::{Confirm, Select};
use ngrammatic::CorpusBuilder;
use std::collections::HashSet;
use std::fs;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::rc::Rc;

// Let the user interactively choose a new project
fn choose_new_project(
    config: Rc<Config>,
    server_store: &ServerStore,
) -> Result<Project, ApplicationError> {
    // If no project name was provided, let the user pick one
    let mut projects = fs::read_dir(&config.get_servers_dir())
        .map_err(|_| ApplicationError::ReadServersDir(config.get_servers_dir()))?
        .filter_map(|result| {
            if let Ok(dir_entry) = result {
                let file_name = dir_entry.file_name();
                let project_name = file_name.to_str()?;
                if server_store.get_one(project_name).is_ok() {
                    // Ignore this project because it is already a server
                    return None;
                }

                return Project::from_name(&config, project_name.to_string()).ok();
            }

            None
        })
        .collect::<Vec<_>>();
    projects.sort_by(|project1, project2| project1.name.cmp(&project2.name));

    if projects.is_empty() {
        return Err(ApplicationError::NoNewProjects(config.get_servers_dir()));
    }

    Select::new("Pick a project", projects)
        .prompt()
        .map_err(ApplicationError::from)
}

// Get an existing server from the command line argument, falling back to letting the user interactively pick one
fn get_existing_server_from_user<'s>(
    server_store: &'s ServerStore,
    cli_server_name: Option<&str>,
    prompt: &str,
) -> Result<&'s Server, ApplicationError> {
    match cli_server_name {
        Some(server_name) => server_store.get_one(server_name),
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

// Get the start command from the script name command line argument, falling back to letting the user interactively pick one
fn get_start_command_from_user(
    project: &Project,
    cli_start_script: Option<&str>,
    prompt: &str,
) -> Result<String, ApplicationError> {
    let start_script = match cli_start_script {
        Some(start_script) => {
            // If a start script name was provided from the command line, validate it
            project.validate_start_script(start_script)?;
            start_script.to_string()
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

fn get_project_dirs() -> Result<ProjectDirs, ApplicationError> {
    ProjectDirs::from("com", "Canac Apps", "Server Room").ok_or(ApplicationError::ProjectDirs)
}

// Return the path to the config file
fn get_config_path() -> Result<PathBuf, ApplicationError> {
    Ok(get_project_dirs()?
        .config_dir()
        .join(PathBuf::from("config.toml")))
}

// Return the path to the server store file
fn get_store_path() -> Result<PathBuf, ApplicationError> {
    Ok(get_project_dirs()?
        .data_dir()
        .join(PathBuf::from("servers.toml")))
}

// Load the configuration and server store
fn load() -> Result<(Rc<Config>, ServerStore), ApplicationError> {
    let config = Rc::new(Config::load(get_config_path()?)?);
    let server_store = ServerStore::load(get_store_path()?, config.clone())?;
    Ok((config, server_store))
}

fn run() -> Result<(), ApplicationError> {
    let app = App::new("server-room")
        // Allow invalid subcommands because we suggest the correct one
        .setting(AppSettings::AllowExternalSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .version("0.1.0")
        .author("Caleb Cox")
        .about("Runs dev servers")
        .subcommand(SubCommand::with_name("cargo").about("Displays configuration"))
        .subcommand(
            SubCommand::with_name("add")
                .about("Adds a new server")
                .arg(
                    Arg::with_name("project-name")
                        .help("Specifies the project name")
                        .takes_value(true)
                        .short("p")
                        .long("project-name"),
                )
                .arg(
                    Arg::with_name("start-script")
                        .help("Sets the new server's start script")
                        .takes_value(true)
                        .short("s")
                        .long("start-script")
                        .requires("project-name"),
                ),
        )
        .subcommand(
            SubCommand::with_name("edit")
                .about("Changes a server's start script")
                .arg(
                    Arg::with_name("server")
                        .help("Specifies the server to edit")
                        .takes_value(true)
                        .short("s")
                        .long("server"),
                )
                .arg(
                    Arg::with_name("start-script")
                        .help("Sets the server's new start script")
                        .takes_value(true)
                        .long("start-script")
                        .requires("server"),
                )
                .arg(
                    Arg::with_name("force")
                        .help("Don't prompt for confirmation")
                        .short("f")
                        .long("force")
                        .requires("start-script"),
                ),
        )
        .subcommand(
            SubCommand::with_name("run").about("Runs a server").arg(
                Arg::with_name("server")
                    .help("Specifies the server to run")
                    .takes_value(true)
                    .short("s")
                    .long("server"),
            ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Removes a server")
                .alias("rm")
                .arg(
                    Arg::with_name("server")
                        .help("Specifies the server to remove")
                        .takes_value(true)
                        .short("s")
                        .long("server"),
                )
                .arg(
                    Arg::with_name("force")
                        .help("Don't prompt for confirmation")
                        .short("f")
                        .long("force")
                        .requires("server"),
                ),
        )
        .subcommand(SubCommand::with_name("list").about("Displays all servers"));
    let matches = app.get_matches();

    match matches.subcommand_name() {
        Some("config") => {
            // Display the config and server store paths without actually loading the config because the load might fail
            // if they don't exist or are malformed
            let config_path = get_config_path()?;
            println!(
                "Configuration file path: {:?}\nServer store path: {:?}",
                config_path,
                get_store_path()?
            );
            if let Ok(config) = Config::load(config_path) {
                println!("servers directory: {:?}", config.get_servers_dir())
            }
            Ok(())
        }
        Some("add") => {
            let (config, server_store) = load()?;
            let options = matches.subcommand_matches("add").unwrap();
            let project = match options.value_of("project-name") {
                Some(project_name) => Project::from_name(&config, project_name.to_string()),
                None => choose_new_project(config, &server_store),
            }?;
            let start_command = get_start_command_from_user(
                &project,
                options.value_of("start-script"),
                "Pick a start script",
            )?;
            server_store.add_server(&project, start_command)
        }
        Some("edit") => {
            let (config, server_store) = load()?;
            let options = matches.subcommand_matches("edit").unwrap();
            let server = get_existing_server_from_user(
                &server_store,
                options.value_of("server"),
                "Pick a server to edit",
            )?;
            let project = Project::from_name(&config, server.name.clone())?;
            let start_command = get_start_command_from_user(
                &project,
                options.value_of("start-script"),
                "Pick a new start script",
            )?;
            if get_confirmation_from_user(
                options.is_present("force"),
                "Are you sure you want to change the server's start script?",
            )? {
                server_store.set_server_start_command(&server.name, start_command)
            } else {
                Ok(())
            }
        }
        Some("run") => {
            let (_, server_store) = load()?;
            let options = matches.subcommand_matches("run").unwrap();
            let server = get_existing_server_from_user(
                &server_store,
                options.value_of("server"),
                "Pick a server to run",
            )?;
            server_store.start_server(&server.name)
        }
        Some("remove") => {
            let (_, server_store) = load()?;
            let options = matches.subcommand_matches("remove").unwrap();
            let server = get_existing_server_from_user(
                &server_store,
                options.value_of("server"),
                "Pick a server to remove",
            )?;
            if get_confirmation_from_user(
                options.is_present("force"),
                "Are you sure you want to remove the server?",
            )? {
                server_store.remove_server(&server.name)
            } else {
                Ok(())
            }
        }
        Some("list") => {
            let (_, server_store) = load()?;
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
        Some(command) => Err(ApplicationError::InvalidCommand(command.to_string())),
        None => panic!("No command specified"),
    }
}

fn main() {
    let exit_code = match run() {
        Ok(_) => 0,
        Err(err) => {
            // Generate user-facing suggestions based on the error
            let suggestion: Option<String> = match &err {
                ApplicationError::ProjectDirs => None,
                ApplicationError::ReadConfig(path) => Some(format!("Try creating a configuration file at {:?}.", path)),
                ApplicationError::ParseConfig(_) => Some("Make sure that the configuration file is valid TOML. Example:\n\nservers_dir = '...'".to_string()),
                ApplicationError::WriteStore(_) => Some("Make sure that the server store file is writable.".to_string()),
                ApplicationError::ParseStore(_) => Some("Make sure that the server store file is valid TOML.".to_string()),
                ApplicationError::StringifyStore => None,
                ApplicationError::ReadServersDir(_) => Some("Try setting `servers_dir` in the configuration to the directory where your servers are.".to_string()),
                ApplicationError::ReadPackageJson(servers_dir) => Some(format!("Try creating a new npm project in this project directory.\n\n    cd {:?}\n    npm init", servers_dir)),
                ApplicationError::MalformedPackageJson { path: _, cause: _ } => Some("Try making sure that your package.json contains valid JSON and that the \"scripts\" property is an object with at least one key. For example:\n\n    \"scripts\": {\n        \"start\": \"node app.js\"\n    }".to_string()),
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
                    let suggested_server = load().ok().and_then(|(_, server_store)| {
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
                ApplicationError::NoNewProjects(servers_dir) => Some(format!("Try creating a new project in \"{:?}\" first.", servers_dir)),
                ApplicationError::NoServers => Some("Try adding a new server first.\n\n    server-room add".to_string()),
                ApplicationError::InquireError(_) => None,
                ApplicationError::InvalidCommand(command) => {
                    let mut corpus = CorpusBuilder::new().finish();
                    corpus.add_text("config");
                    corpus.add_text("add");
                    corpus.add_text("edit");
                    corpus.add_text("run");
                    corpus.add_text("remove");
                    corpus.add_text("list");
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
