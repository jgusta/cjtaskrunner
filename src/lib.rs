use std::fmt;
use std::io;

#[derive(Debug)]
pub struct CjError {
    message: String,
}

impl CjError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CjError {}

impl From<io::Error> for CjError {
    fn from(value: io::Error) -> Self {
        Self::new(value.to_string())
    }
}

pub type CjResult<T> = Result<T, CjError>;

mod ansi;
mod cli;
mod command_text;
mod directive_info;
mod directives;
mod environment;
mod formatter;
mod help_output;
pub mod lsp;
mod project_init;
mod runner;
mod runtime;
mod task_file;
mod taskfile_discovery;
mod version;

pub use cli::run_cli;
pub use task_file::{parse_task_file, TaskFile};

#[cfg(test)]
mod lib_tests;
