use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Couldn't read servers directory \"{0}\"")]
    ReadServersDir(PathBuf),

    #[error("Could not read file \"{0}\"")]
    ReadPackageJson(PathBuf),

    #[error("Malformed package.json file \"{path}\": {cause}")]
    MalformedPackageJson { path: PathBuf, cause: String },

    #[error("Script \"{script}\" doesn't exist in \"{path}\"")]
    NonExistentScript { path: PathBuf, script: String },

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

    #[error("Invalid command \"{0:?}\"")]
    InvalidCommand(String),
}
