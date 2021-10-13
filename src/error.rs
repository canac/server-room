use super::project::Project;
use super::server::Server;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Couldn't determine application directories")]
    ProjectDirs,

    #[error("Couldn't write server store file \"{0}\"")]
    WriteStore(PathBuf),

    #[error("Couldn't parse server store file \"{0}\"")]
    ParseStore(PathBuf),

    #[error("Couldn't stringify server store")]
    StringifyStore,

    #[error("Could not read file \"{0}\"")]
    ReadPackageJson(PathBuf),

    #[error("Malformed package.json file \"{path}\": {cause}")]
    MalformedPackageJson { path: PathBuf, cause: String },

    #[error("Couldn't parse path \"{0}\"")]
    ParsePath(PathBuf),

    #[error("Script \"{script}\" doesn't exist in \"{:?}\"", .project.get_package_json())]
    NonExistentScript { project: Project, script: String },

    #[error("Couldn't execute command \"{0}\"")]
    RunScript(String),

    #[error("Server \"{0}\" don't exist")]
    NonExistentServer(String),

    #[error("Server with name \"{0}\" already exists")]
    DuplicateServerName(String),

    #[error("Server at \"{dir}\" already exists")]
    DuplicateServerDir { dir: PathBuf, existing: Server },

    #[error("No servers have been added yet")]
    NoServers,

    #[error(transparent)]
    InquireError(#[from] inquire::error::InquireError),

    #[error("Invalid command \"{0}\"")]
    InvalidCommand(String),
}
