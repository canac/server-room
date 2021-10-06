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
use directories::ProjectDirs;
use inquire::Select;
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

fn run() -> Result<(), ApplicationError> {
    let config = Rc::new(Config::load(get_config_path()?)?);
    let server_store = ServerStore::load(get_store_path()?, config.clone())?;

    let app = App::new("server-room")
        // Allow invalid subcommands because we suggest the correct one
        .setting(AppSettings::AllowExternalSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .version("0.1.0")
        .author("Caleb Cox")
        .about("Runs dev servers")
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
                ),
        );
    let matches = app.get_matches();

    match matches.subcommand_name() {
        Some("add") => {
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
            server_store.set_server_start_command(&server.name, start_command)
        }
        Some("run") => {
            let options = matches.subcommand_matches("run").unwrap();
            let server = get_existing_server_from_user(
                &server_store,
                options.value_of("server"),
                "Pick a server to run",
            )?;
            server_store.start_server(&server.name)
        }
        Some("remove") => {
            let options = matches.subcommand_matches("remove").unwrap();
            let server = get_existing_server_from_user(
                &server_store,
                options.value_of("server"),
                "Pick a server to remove",
            )?;
            server_store.remove_server(&server.name)
        }
        Some(command) => Err(ApplicationError::InvalidCommand(command.to_string())),
        None => panic!("No command specified"),
    }
}

// Generate a suggestion for the closest server name that matches the provided server name
// Returns Err if the server store couldn't be loaded. Returns Ok(None) if no suggestions were found.
fn suggest_server_name(server_name: &str) -> Result<Option<String>, ApplicationError> {
    let config = Config::load(get_config_path()?)?;
    let server_store = ServerStore::load(get_store_path()?, Rc::new(config))?;
    Ok(server_store.get_closest_server_name(server_name))
}

fn main() {
    let exit_code = match run() {
        Ok(_) => 0,
        Err(err) => {
            // Generate user-facing suggestions based on the error
            let suggestion: Option<String> = match &err {
                ApplicationError::ProjectDirs => None,
                ApplicationError::ReadConfig(_) => Some("Make sure that the configuration file exists.".to_string()),
                ApplicationError::ParseConfig(_) => Some("Make sure that the configuration file is valid TOML.".to_string()),
                ApplicationError::ReadStore(_) => Some("Make sure that the server store file exists.".to_string()),
                ApplicationError::WriteStore(_) => Some("Make sure that the server store file is writable.".to_string()),
                ApplicationError::ParseStore(_) => Some("Make sure that the server store file is valid TOML.".to_string()),
                ApplicationError::StringifyStore => None,
                ApplicationError::ReadServersDir(_) => Some("Try setting `servers_dir` in the configuration to the directory where your servers are.".to_string()),
                ApplicationError::ReadPackageJson(servers_dir) => Some(format!("Try creating a new npm project in this project directory.\n\n    cd {:?}\n    npm init", servers_dir)),
                ApplicationError::MalformedPackageJson { path: _, cause: _ } => Some("Try making sure that your package.json contains valid JSON and that the \"scripts\" property is an object with at least one key. For example:\n\n    \"scripts\": {\n        \"start\": \"node app.js\"\n    }".to_string()),
                ApplicationError::NonExistentScript {
                    path: _,
                    script,
                } => Some(format!("Try adding the script {} to your package.json.", script)),
                ApplicationError::RunScript(_) => Some("Make sure that the command is spelled correctly and is in the path.".to_string()),
                ApplicationError::NonExistentServer(server) => {
                    Some(match suggest_server_name(server) {
                        Ok(Some(suggestion)) => format!("Did you mean --server {}?", suggestion),
                        Err(_) | Ok(None) => "Try a different server name.".to_string(),
                    })
                },
                ApplicationError::DuplicateServer(server) => Some(format!(
                    "Try editing the existing server instead.\n\n    server-room edit --server {}",
                    server
                )),
                ApplicationError::NoNewProjects(servers_dir) => Some(format!("Try creating a new project in \"{:?}\" first.", servers_dir)),
                ApplicationError::NoServers => Some("Try adding a new server first.\n\n    server-room add".to_string()),
                ApplicationError::InquireError(_) => None,
                ApplicationError::InvalidCommand(command) => {
                    let mut corpus = CorpusBuilder::new().finish();
                    corpus.add_text("add");
                    corpus.add_text("edit");
                    corpus.add_text("run");
                    corpus.add_text("remove");
                    let results = corpus.search(command.as_str(), 0.5f32);
                    Some(match results.first() {
                        Some(result) => format!("Did you mean `server-room {}`?", result.text),
                        None => "Try `server-room --help` to see available subcommands.".to_string(),
                    })
                }
            };

            eprintln!("{}", err);
            if let Some(suggestion) = suggestion {
                eprintln!("{}", suggestion);
            }
            1
        }
    };

    std::process::exit(exit_code);
}
