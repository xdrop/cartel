use crate::shell::interactive_shell_cmd_line;
use anyhow::Result;
use log::{debug, info};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

pub struct CurrentEnvHolder {
    environment: RwLock<HashMap<String, String>>,
}

impl Default for CurrentEnvHolder {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentEnvHolder {
    pub fn new() -> CurrentEnvHolder {
        CurrentEnvHolder {
            environment: RwLock::new(HashMap::new()),
        }
    }
    pub fn replace(&self, new_env: HashMap<String, String>) {
        *self.environment.write() = new_env;
    }

    pub fn read(&self) -> HashMap<String, String> {
        self.environment.read().clone()
    }
}

pub fn grab_env() -> Result<HashMap<String, String>> {
    let cmd = interactive_shell_cmd_line(false)?;
    let (head, tail) = cmd.split_first().expect("Empty command in grab_env");
    let print_env = vec!["-c", "printenv; exit 0"];
    let output = Command::new(head).args(tail).args(print_env).output()?;
    Ok(parse_printenv_output(&output.stdout))
}

fn parse_printenv_output(output: &[u8]) -> HashMap<String, String> {
    let reader = BufReader::new(output);
    let mut env = HashMap::new();
    let lines = reader.lines().map_while(Result::ok);
    for line in lines {
        let (key, val) =
            line.split_once('=').expect("Unexpected printenv output");
        env.insert(key.to_owned(), val.to_owned());
    }
    env
}

/// Launches a thread which fetches the users environment variables from an
/// interactive login shell.
///
/// This is done by creating a child shell process (typically with --login and -i)
/// of the currently running shell. The process executes `printenv` and exits,
/// and its results are parsed and stored in the env holder map.
///
/// The map is protected by a `RwLock` and its being written to approximately
/// every 5 seconds.
///
/// The main use case for this is to reflect changes to the shells
/// initialisation files (eg. zshrc) without requiring the user to restart the
/// daemon, and prevent issues arising from a stale set of environment
/// variables.
///
/// This feature is still experimental, and may be removed if it does not
/// provide substantial benefit to offset the complexity it introduces.
pub fn env_grabber_thread(current_env_holder: Arc<CurrentEnvHolder>) {
    info!("Starting env-grabber thread");
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(5000));
        debug!("env-grab started");
        let new_env = grab_env().expect("failed");
        current_env_holder.replace(new_env);
        debug!("env-grab done");
    });
}
