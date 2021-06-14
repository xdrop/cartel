use std::ffi::OsString;
use thiserror::Error;

/// Daemon enumerates all possible errors returned by this library.
#[derive(Error, Debug)]
pub enum DaemonError {
    /// Represents the case when a requested module was not found. For
    /// example, trying to stop a module that doesn't exist.
    #[error("Module with name '{0}' not found")]
    NotFound(String),

    /// Represents the case when attempting to stop a module that is not
    /// running or doesn't exist.
    #[error("Module with name '{0}' is not running or doesn't exist.")]
    NotRunning(String),

    /// Represents the case some of the module in the given subset of
    /// modules do not exist. For example, trying to deploy a set of modules
    /// where one doesn't exist.
    #[error("Module not found")]
    SubsetNotFound,

    #[error("Task {task_name:?} failed with exit code {code:?}. Use \"cartel logs {task_name}\" \
     or view {log_file:?} for more details.")]
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
