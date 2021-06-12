/// Prepare a command list for executing the given shell line.
///
/// This will always call out to bash.
pub fn shell_to_cmd(shell_cmd: &str) -> Vec<String> {
    vec!["/bin/bash", "-c", shell_cmd]
        .into_iter()
        .map(String::from)
        .collect()
}
