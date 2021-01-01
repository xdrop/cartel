use std::ffi::OsString;
use thiserror::Error;

/// Daemon enumerates all possible errors returned by this library.
#[derive(Error, Debug)]
pub enum DaemonError {
    /// Represents the case when a requested resource was not found. For
    /// example, trying to stop a module that doesn't exist.
    #[error("Resource with name `[{0}]` not found")]
    NotFound(String),

    /// Represents the case some of the resources in the given subset of
    /// resources do not exist. For example, trying to deploy a set of modules
    /// where one doesn't exist.
    #[error("Resource not found")]
    SubsetNotFound,

    #[error("Task {task_name:?} failed with code {code:?}. See {log_file:?} for details")]
    TaskFailed {
        task_name: String,
        code: i32,
        log_file: OsString,
    },

    /// Represents a failure to read from input.
    #[error("Read error")]
    ReadError { source: std::io::Error },

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}
