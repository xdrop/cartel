/// Attempts to infer a path to the current running shell.
///
/// Anything outside zsh, bash, or fish results to `None`. Otherwise the name of
/// the shell is prefixed with `/bin` and returned.
pub fn active_shell_path() -> Option<String> {
    let current_shell =
        std::env::var("SHELL").expect("Failed to get current shell");

    if current_shell.contains("zsh") {
        Some(String::from("/bin/zsh"))
    } else if current_shell.contains("bash") {
        Some(String::from("/bin/bash"))
    } else if current_shell.contains("fish") {
        Some(String::from("/bin/fish"))
    } else {
        None
    }
}
