mod config;
mod script;
mod server;

use config::Config;
use script::Script;
use server::Server;

use clap::{App, Arg, SubCommand};
use inquire::Select;
use std::collections::HashSet;
use std::fs;
use std::iter::FromIterator;

// Get the name of a new project from the command line argument, falling back to letting the user interactively pick one
fn get_new_project_name_from_user(
    config: &Config,
    cli_project_name: Option<&str>,
) -> Result<String, String> {
    match cli_project_name {
        Some(project_name) => {
            // If a project name was provided from the command line, validate it
            let project_name = project_name.to_string();
            config.validate_new_project_name(&project_name)?;
            Ok(project_name)
        }
        None => {
            // If no project name was provided, let the user pick one
            let projects = fs::read_dir(&config.servers_dir)
                .map_err(|_| "Error reading servers directory".to_string())?
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
            Select::new("Pick a project", projects)
                .prompt()
                .map_err(|err| err.to_string())
        }
    }
}

// Get an existing server from the command line argument, falling back to letting the user interactively pick one
fn get_existing_server_from_user<'a>(
    config: &'a Config,
    cli_project_name: Option<&str>,
    prompt: &str,
) -> Result<&'a Server, String> {
    match cli_project_name {
        Some(project_name) => {
            // If a server was provided from the command line, validate it
            config
                .servers
                .get(&project_name.to_string())
                .ok_or(format!("Server \"{}\" does not exist", project_name))
        }
        None => {
            // If no server was provided, let the user pick one
            let mut servers = config.servers.values().collect::<Vec<_>>();
            servers.sort_by_key(|server| !server.get_weight());
            Select::new(prompt, servers)
                .prompt()
                .map_err(|err| err.to_string())
        }
    }
}

// Get the start command from the script name command line argument, falling back to letting the user interactively pick one
fn get_start_command_from_user(
    config: &Config,
    project_name: &str,
    cli_start_script: Option<&str>,
    prompt: &str,
) -> Result<String, String> {
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
            let start_script = Select::new(prompt, scripts)
                .prompt()
                .map_err(|err| err.to_string())?;
            start_script.name
        }
    };
    Ok(format!("npm run {}", start_script))
}

fn main() -> Result<(), String> {
    let config = Config::load_config();

    let matches = App::new("server-room")
        .version("0.1.0")
        .author("Caleb Cox")
        .about("Runs dev servers")
        .subcommand(
            SubCommand::with_name("add")
                .about("add a new server")
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
                .about("edit a server's start script")
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
            SubCommand::with_name("run").about("run a server").arg(
                Arg::with_name("server")
                    .help("Specifies the server to run")
                    .takes_value(true)
                    .short("s")
                    .long("server"),
            ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("remove a server")
                .alias("rm")
                .arg(
                    Arg::with_name("server")
                        .help("Specifies the server to remove")
                        .takes_value(true)
                        .short("s")
                        .long("server"),
                ),
        )
        .get_matches();

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
        _ => return Err("Some other subcommand was used".to_string()),
    }

    Ok(())
}
