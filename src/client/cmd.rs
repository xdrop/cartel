use crate::shell::interactive_shell_cmd_line;

/// Prepare a command list for executing the given shell line.
///
/// This will always call out to bash.
pub fn shell_to_cmd(shell_cmd: &str) -> Vec<String> {
    vec!["/bin/bash", "-c", shell_cmd]
        .into_iter()
        .map(String::from)
        .collect()
}

/// Prepare a command list to be run in a shell.
///
/// This will always call out to bash.
pub fn cmd_in_shell(cmd: &[&str]) -> Vec<String> {
    let joined = cmd.join(" ");
    shell_to_cmd(&joined)
}

/// Prepare a command list for executing the given shell line in an interactive
/// login shell.
pub fn shell_to_cmd_interactive(shell_cmd: &str) -> Vec<String> {
    let mut interactive_shell_cmd =
        interactive_shell_cmd_line(false).expect("Unexpected shell");
    interactive_shell_cmd.push(String::from("-c"));
    interactive_shell_cmd.push(String::from(shell_cmd));
    interactive_shell_cmd
}
