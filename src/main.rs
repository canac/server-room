mod actionable_error;
mod config;
mod script;
mod server;

use actionable_error::{ActionableError, ErrorCode};
use config::Config;
use ngrammatic::CorpusBuilder;
use script::Script;
use server::Server;

use clap::{App, AppSettings, Arg, SubCommand};
use inquire::Select;
use std::collections::HashSet;
use std::fs;
use std::iter::FromIterator;

// Get the name of a new project from the command line argument, falling back to letting the user interactively pick one
fn get_new_project_name_from_user(
    config: &Config,
    cli_project_name: Option<&str>,
) -> Result<String, ActionableError> {
    match cli_project_name {
        Some(project_name) => {
            // If a project name was provided from the command line, validate it
            let project_name = project_name.to_string();
            config.validate_new_project_name(&project_name)?;
            Ok(project_name)
        }
        None => {
            // If no project name was provided, let the user pick one
            let mut projects = fs::read_dir(&config.servers_dir)
                .map_err(|_| ActionableError {
                    code: ErrorCode::ReadServersDir,
                    message: format!("Couldn't read servers directory {}", config.servers_dir),
                    suggestion: "Try setting `servers_dir` in the configuration to the directory where your servers are.".to_string(),
                })?
                .filter_map(|result| {
                    if let Ok(dir_entry) = result {
                        let project_name = dir_entry.file_name().to_str()?.to_string();
                        if config.validate_new_project_name(&project_name).is_ok() {
                            return Some(project_name);
                        }
                    }

                    None
                })
                .collect::<Vec<_>>();
            projects.sort();
            Select::new("Pick a project", projects)
                .prompt()
                .map_err(ActionableError::from)
        }
    }
}

// Get an existing server from the command line argument, falling back to letting the user interactively pick one
fn get_existing_server_from_user<'a>(
    config: &'a Config,
    cli_project_name: Option<&str>,
    prompt: &str,
) -> Result<&'a Server, ActionableError> {
    match cli_project_name {
        Some(project_name) => {
            // If a server was provided from the command line, validate it
            config
                .servers
                .get(&project_name.to_string())
                .ok_or_else(|| {
                    let suggestion = match config.get_closest_server_name(project_name) {
                        Some(suggested_server_name) => {
                            format!("Did you mean --server {}?", suggested_server_name)
                        }
                        None => "Try a different server name.".to_string(),
                    };
                    ActionableError {
                        code: ErrorCode::NonExistentServer,
                        message: format!("Server \"{}\" does not exist", project_name),
                        suggestion,
                    }
                })
        }
        None => {
            // If no server was provided, let the user pick one
            let mut servers = config.servers.values().collect::<Vec<_>>();
            // Put the servers with the highest weight first
            servers.sort_by(|server1, server2| {
                server1
                    .get_weight()
                    .partial_cmp(&server2.get_weight())
                    .unwrap()
                    .reverse()
            });
            Select::new(prompt, servers)
                .prompt()
                .map_err(ActionableError::from)
        }
    }
}

// Get the start command from the script name command line argument, falling back to letting the user interactively pick one
fn get_start_command_from_user(
    config: &Config,
    project_name: &str,
    cli_start_script: Option<&str>,
    prompt: &str,
) -> Result<String, ActionableError> {
    let start_script = match cli_start_script {
        Some(start_script) => {
            // If a start script name was provided from the command line, validate it
            let start_script = start_script.to_string();
            config.validate_start_script(project_name, &start_script)?;
            start_script
        }
        None => {
            // If no start script was provided, let the user pick one
            let mut scripts = config.load_project_start_scripts(project_name)?;
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

fn run() -> Result<(), ActionableError> {
    let config = Config::load_config();

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
            let project_name =
                get_new_project_name_from_user(&config, options.value_of("project-name"))?;
            let start_command = get_start_command_from_user(
                &config,
                &project_name,
                options.value_of("start-script"),
                "Pick a start script",
            )?;
            config.add_server(project_name, start_command);
        }
        Some("edit") => {
            let options = matches.subcommand_matches("edit").unwrap();
            let server = get_existing_server_from_user(
                &config,
                options.value_of("server"),
                "Pick a server to edit",
            )?;
            let start_command = get_start_command_from_user(
                &config,
                &server.name,
                options.value_of("start-script"),
                "Pick a new start script",
            )?;
            config.set_server_start_command(&server.name, start_command);
        }
        Some("run") => {
            let options = matches.subcommand_matches("run").unwrap();
            let server = get_existing_server_from_user(
                &config,
                options.value_of("server"),
                "Pick a server to run",
            )?;
            server.start(&config);
        }
        Some("remove") => {
            let options = matches.subcommand_matches("remove").unwrap();
            let server = get_existing_server_from_user(
                &config,
                options.value_of("server"),
                "Pick a server to remove",
            )?;
            config.remove_server(server);
        }
        Some(command) => {
            let mut corpus = CorpusBuilder::new().finish();
            corpus.add_text("add");
            corpus.add_text("edit");
            corpus.add_text("run");
            corpus.add_text("remove");
            let results = corpus.search(command, 0.5f32);
            let suggestion = match results.first() {
                Some(result) => format!("Did you mean `server-room {}`?", result.text),
                None => "Try `server-room --help` to see available subcommands.".to_string(),
            };

            return Err(ActionableError {
                code: ErrorCode::InvalidCommand,
                message: format!("Invalid command {}", command),
                suggestion,
            });
        }
        None => panic!("No command specified"),
    }

    Ok(())
}

fn main() {
    let exit_code = match run() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    };

    std::process::exit(exit_code);
}
