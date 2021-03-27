#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;

/// Bootstrap the current process into a shell.
///
/// This replaces the current process by a shell (using the `exec` syscall),
/// whose shell arguments attempt to spawn another instance of the daemon.
///
/// Running the daemon into a shell allows us to inherit PATH and makes more
/// convoluted setups easier to execute.
///
/// The only shells that have been tested are bash and zsh that both support the
/// `-c` option, which is used to recreate the process.
///
/// Currently none of the args are passed down (otherwise this would lead to
/// infinite processes) as there is only a single daemon arg (to spawn the shell
/// itself) but in the future we could consider selectively propagating some
/// args.
pub fn bootstrap_shell(shell_path: String) {
    let current_exe_path = std::env::current_exe()
        .expect("Failed to bootstrap into a shell")
        .into_os_string()
        .into_string()
        .expect("Failed to get current exe path");

    let mut extra_args = vec![];
    // Zsh-specific configuration
    if shell_path.contains("zsh") {
        extra_args.push("--login");
        extra_args.push("-i");
    }

    // Bash-specific configuration
    if shell_path.contains("bash") {
        extra_args.push("--login");
        extra_args.push("-i");
    }

    // Fish-specific configuration
    if shell_path.contains("fish") {
        extra_args.push("--login");
        extra_args.push("-i");
    }
    let cmd = format!("{} || exit 1", current_exe_path);
    let args = vec!["-c", &cmd];

    // Replace the current process
    Command::new(shell_path).args(extra_args).args(args).exec();
}
