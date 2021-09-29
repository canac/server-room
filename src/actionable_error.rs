use std::fmt::Display;

#[derive(Debug)]
pub struct ActionableError {
    pub code: ErrorCode,
    pub message: String,
    pub suggestion: String,
}

impl Display for ActionableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}\n\n{}", self.code, self.message, self.suggestion)
    }
}

impl std::error::Error for ActionableError {}

impl From<inquire::error::InquireError> for ActionableError {
    fn from(error: inquire::error::InquireError) -> Self {
        ActionableError {
            code: ErrorCode::InquireError,
            message: error.to_string(),
            suggestion: "Try again.".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum ErrorCode {
    ReadServersDir,
    ReadPackageJson,
    ParsePackageJson,
    MissingStartScript,
    NonExistentServer,
    DuplicateProject,
    NoNewServers,
    NoServers,
    InquireError,
    InvalidCommand,
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
