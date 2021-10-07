use super::project::Project;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Couldn't determine application directories")]
    ProjectDirs,

    #[error("Couldn't read config file \"{0}\"")]
    ReadConfig(PathBuf),

    #[error("Couldn't parse config file \"{0}\"")]
    ParseConfig(PathBuf),

    #[error("Couldn't write server store file \"{0}\"")]
    WriteStore(PathBuf),

    #[error("Couldn't parse server store file \"{0}\"")]
    ParseStore(PathBuf),

    #[error("Couldn't stringify server store")]
    StringifyStore,

    #[error("Couldn't read servers directory \"{0}\"")]
    ReadServersDir(PathBuf),

    #[error("Could not read file \"{0}\"")]
    ReadPackageJson(PathBuf),

    #[error("Malformed package.json file \"{path}\": {cause}")]
    MalformedPackageJson { path: PathBuf, cause: String },

    #[error("Script \"{script}\" doesn't exist in \"{:?}\"", .project.get_package_json())]
    NonExistentScript { project: Project, script: String },

    #[error("Couldn't execute command \"{0}\"")]
    RunScript(String),

    #[error("Server \"{0}\" don't exist")]
    NonExistentServer(String),

    #[error("Server \"{0}\" already exists")]
    DuplicateServer(String),

    #[error("Servers directory \"{0}\" only contains existing servers")]
    NoNewProjects(PathBuf),

    #[error("No servers have been added yet")]
    NoServers,

    #[error(transparent)]
    InquireError(#[from] inquire::error::InquireError),

    #[error("Invalid command \"{0}\"")]
    InvalidCommand(String),
}
