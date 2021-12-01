use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
pub enum Edit {
    Name {
        #[structopt(short, long, about = "Specifies the server to edit")]
        server: Option<String>,
        #[structopt(long, requires = "server", about = "Specifies the server's new name")]
        name: Option<String>,
        #[structopt(short, long, about = "Don't prompt for confirmation")]
        force: bool,
    },

    StartScript {
        #[structopt(short, long, about = "Specifies the server to edit")]
        server: Option<String>,
        #[structopt(
            long,
            requires = "server",
            about = "Specifies the server's new start script"
        )]
        start_script: Option<String>,
        #[structopt(short, long, about = "Don't prompt for confirmation")]
        force: bool,
    },

    Port {
        #[structopt(short, long, about = "Specifies the server to edit")]
        server: Option<String>,
        #[structopt(long, requires = "server", about = "Specifies the server's new port")]
        port: Option<u16>,
        #[structopt(short, long, about = "Don't prompt for confirmation")]
        force: bool,
    },
}

#[derive(StructOpt)]
#[structopt(
    name = "server-room",
    about = "Runs dev servers",
    version = "0.1.0",
    author = "Caleb Cox"
)]
pub enum Cli {
    #[structopt(about = "Displays configuration")]
    Config,

    #[structopt(about = "Adds a new server")]
    Add {
        #[structopt(parse(from_os_str), about = "Specifies the project path")]
        path: PathBuf,
        #[structopt(short, long, about = "Specifies the project name")]
        name: Option<String>,
        #[structopt(short, long, about = "Specifies the new server's start script")]
        start_script: Option<String>,
        #[structopt(long, about = "Specifies the new server's port")]
        port: Option<u16>,
    },

    #[structopt(about = "Changes a server's definition")]
    Edit(Edit),

    #[structopt(about = "Runs a server")]
    Run {
        #[structopt(short, long, about = "Specifies the server to run")]
        server: Option<String>,
    },

    #[structopt(alias = "rm", about = "Removes a server")]
    Remove {
        #[structopt(short, long, about = "Specifies the server to remove")]
        server: Option<String>,
        #[structopt(
            short,
            long,
            requires = "server",
            about = "Don't prompt for confirmation"
        )]
        force: bool,
    },

    #[structopt(alias = "ls", about = "Displays all servers")]
    List,

    #[structopt(about = "Generates a Caddyfile")]
    Caddy,

    #[structopt(external_subcommand)]
    Unknown(Vec<String>),
}
