mod config;
mod script;
mod server;

use config::Config;
use script::Script;
use server::Server;

use clap::{App, Arg, SubCommand};
use inquire::Select;
use std::fs;

// Let the user pick a server from the defined list in the config
fn pick_server(config: &mut Config) -> &mut Server {
    Select::new("Pick a server", config.servers.iter_mut().collect())
        .prompt()
        .unwrap()
}

// Pick a new project from the servers directory that isn't a server yet
fn pick_project(config: &Config) -> String {
    let projects = fs::read_dir(&config.servers_dir)
        .expect("Error reading servers directory")
        .filter_map(|result| {
            if let Ok(dir_entry) = result {
                let project_name = dir_entry.file_name().to_str().unwrap().to_string();
                if config.validate_new_project_name(&project_name).is_ok() {
                    Some(project_name)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    Select::new("Pick a project", projects).prompt().unwrap()
}

// Pick a start script for a particular project
fn pick_start_script(config: &Config, project_name: &String) -> Script {
    Select::new(
        "Pick a start command",
        config.load_project_start_scripts(project_name).unwrap(),
    )
    .prompt()
    .unwrap()
}

fn main() {
    let mut config = Config::load_config();

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
        .subcommand(SubCommand::with_name("run").about("run a server"))
        .get_matches();

    match matches.subcommand_name() {
        Some("add") => {
            let options = matches.subcommand_matches("add").unwrap();
            let project_name = match options.value_of("project-name") {
                Some(project_name) => {
                    let project_name = project_name.to_string();
                    if let Err(msg) = config.validate_new_project_name(&project_name) {
                        eprint!("{}", msg);
                        return;
                    }

                    project_name
                }
                None => pick_project(&config),
            };
            let start_script = match options.value_of("start-script") {
                Some(start_script) => {
                    let start_script = start_script.to_string();
                    if let Err(msg) = config.validate_start_script(&project_name, &start_script) {
                        eprint!("{}", msg);
                        return;
                    }

                    start_script
                }
                None => format!("npm run {}", pick_start_script(&config, &project_name).name),
            };
            config.add_server(Server::new(project_name, start_script));
        }
        Some("run") => {
            let server = pick_server(&mut config);
            server.start();
            config.flush_config();
        }
        _ => println!("Some other subcommand was used"),
    }
}
