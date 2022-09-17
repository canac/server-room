use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Edit {
    /// Edit the server's name
    Name {
        /// Specifies the server to edit
        #[clap(short, long)]
        server: Option<String>,
        /// Specifies the server's new name
        #[clap(long, requires = "server")]
        name: Option<String>,
        /// Don't prompt for confirmation
        #[clap(short, long)]
        force: bool,
    },

    /// Edit the server's start script
    StartScript {
        /// Specifies the server to edit
        #[clap(short, long)]
        server: Option<String>,
        /// Specifies the server's new start script
        #[clap(long, requires = "server")]
        start_script: Option<String>,
        /// Don't prompt for confirmation,
        #[clap(short, long)]
        force: bool,
    },

    /// Edit the server's port
    Port {
        /// Specifies the server to edit
        #[clap(short, long)]
        server: Option<String>,
        /// Specifies the server's new port
        #[clap(long, requires = "server")]
        port: Option<u16>,
        /// Don't prompt for confirmation
        #[clap(short, long)]
        force: bool,
    },
}

#[derive(Parser)]
#[clap(about, version, author)]
pub enum Cli {
    /// Displays configuration
    Config,

    /// Adds a new server
    Add {
        /// Specifies the project path
        #[clap(parse(from_os_str))]
        path: PathBuf,
        /// Specifies the project name
        #[clap(short, long)]
        name: Option<String>,
        /// Specifies the new server's start script
        #[clap(short, long)]
        start_script: Option<String>,
        /// Specifies the new server's port
        #[clap(long, requires = "server")]
        port: Option<u16>,
    },

    /// Changes a server's definition
    #[clap(subcommand)]
    Edit(Edit),

    /// Runs a server
    Run {
        /// Specifies the server to run
        #[clap(short, long)]
        server: Option<String>,
    },

    /// Removes a server
    #[clap(alias = "rm")]
    Remove {
        /// Specifies the server to remove
        #[clap(short, long)]
        server: Option<String>,
        /// Don't prompt for confirmation
        #[clap(short, long, requires = "server")]
        force: bool,
    },

    /// Displays all servers
    #[clap(alias = "ls")]
    List,

    /// Generates a Caddyfile
    Caddy,

    #[clap(external_subcommand)]
    Unknown(Vec<String>),
}
