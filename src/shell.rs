use anyhow::Result;
use std::env::VarError;

/// Returns the name of the active shell.
///
/// This is being read from the `SHELL` environment variable, and assumes this
/// process is run within a shell. If not this returns `Err`.
pub fn active_shell() -> Result<String> {
    std::env::var("SHELL").map_err(VarError::into)
}

/// Attempts to infer a path to the current running shell.
///
/// Anything outside zsh, bash, or fish results to `None`. Otherwise the name of
/// the shell is prefixed with `/bin` and returned.
pub fn active_shell_path() -> Option<String> {
    let current_shell = match active_shell() {
        Ok(s) => s,
        Err(_) => return None,
    };

    if current_shell.contains("zsh")
        || current_shell.contains("bash")
        || current_shell.contains("fish")
    {
        Some(current_shell)
    } else {
        None
    }
}

/// Get an interactive login shell command line of the current shell.
///
/// This function attempts to infer the currently running shell using the
/// `SHELL` environment variable, and constructs a list of command line
/// arguments with which the shell can be invoked in login + interactive mode.
pub fn interactive_shell_cmd_line(login: bool) -> Result<Vec<String>> {
    let current_shell = active_shell()?;
    let shell_path = active_shell_path().expect("Unexpected shell");
    let mut cmd_line = vec![shell_path];

    if current_shell.contains("zsh")
        || current_shell.contains("bash")
        || current_shell.contains("fish")
    {
        if login {
            cmd_line.push("--login".to_string());
        }
        cmd_line.push("-i".to_string());
    }

    Ok(cmd_line)
}
