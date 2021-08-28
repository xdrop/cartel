use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct CommandBuilder {
    command: Command,
}

/// Utility for building [Command] objects.
///
/// This allows some current and future convienences for creating child
/// processes to run.
impl CommandBuilder {
    /// Create a new [CommandBuilder] from the given command.
    ///
    /// The first element in the slice should be the path to the executable,
    /// followed by a list of arguments.
    ///
    /// This function will panic if command is empty.
    pub fn new(command: &[String]) -> Self {
        let (head, tail) = command
            .split_first()
            .expect("Empty command in CommandBuilder::cmd");

        let mut command = Command::new(head);
        command.args(tail);

        CommandBuilder { command }
    }

    /// Set the process's environment variables.
    pub fn env<'c>(
        &'c mut self,
        env: &HashMap<String, String>,
    ) -> &'c mut Self {
        self.command.envs(env);
        self
    }

    /// Set the process's standard output (stdout) handle.
    pub fn stdout<T>(&mut self, stdout: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.command.stdout(stdout);
        self
    }

    /// Set the process's standard error (stderr) handle.
    pub fn stderr<T>(&mut self, stderr: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.command.stderr(stderr);
        self
    }

    /// Set the process's standard output (stdout) stream to be ignored.
    pub fn stdout_null(&mut self) -> &mut Self {
        self.command.stdout(Stdio::null());
        self
    }

    /// Set the process's standard error (stderr) stream to be ignored.
    pub fn stderr_null(&mut self) -> &mut Self {
        self.command.stderr(Stdio::null());
        self
    }

    /// Set the process's standard output (stdout) handle from a [File].
    pub fn stdout_file(&mut self, stdout: File) -> &mut Self {
        self.command.stdout(Stdio::from(stdout));
        self
    }

    /// Set the process's standard error (stderr) handle from a [File].
    pub fn stderr_file(&mut self, stderr: File) -> &mut Self {
        self.command.stderr(Stdio::from(stderr));
        self
    }

    /// Set the process's working directory.
    pub fn work_dir<'c, P>(&'c mut self, work_dir: Option<P>) -> &'c mut Self
    where
        P: AsRef<Path>,
    {
        if let Some(path) = work_dir {
            self.command.current_dir(path);
        }
        self
    }

    /// Consume the built [Command] object and return it.
    pub fn build(self) -> Command {
        self.command
    }
}
