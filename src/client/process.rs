use crate::client::module::{
    CheckDefinition, ServiceOrTaskDefinition, ShellDefinition,
};
use crate::path;
use anyhow::{Context, Result};
use std::process::{Command, ExitStatus, Stdio};

pub fn run_task(task_definition: &ServiceOrTaskDefinition) -> Result<()> {
    let working_dir = task_definition
        .working_dir
        .as_deref()
        .and_then(path::from_user_str);

    let cmd_line = task_definition.cmd_line();
    let mut cmd = Command::new(&cmd_line[0]);

    cmd.args(&cmd_line[1..]).envs(&task_definition.environment);

    if let Some(path) = working_dir {
        cmd.current_dir(path);
    }
    cmd.spawn()
        .with_context(|| {
            format!("Unable to run task '{}'", task_definition.name)
        })?
        .wait()?;

    Ok(())
}

pub fn run_check(check_definition: &CheckDefinition) -> Result<ExitStatus> {
    let working_dir = check_definition
        .working_dir
        .as_deref()
        .and_then(path::from_user_str);

    let cmd_line = check_definition.cmd_line();
    let mut cmd = Command::new(&cmd_line[0]);

    cmd.args(&cmd_line[1..]).stdout(Stdio::null());

    if let Some(path) = working_dir {
        cmd.current_dir(path);
    }
    let check_result = cmd
        .spawn()
        .with_context(|| {
            format!("Failed to run check '{}'", check_definition.name)
        })?
        .wait()?;

    Ok(check_result)
}

pub fn run_shell(shell_definition: &ShellDefinition) -> Result<()> {
    let working_dir = shell_definition
        .working_dir
        .as_deref()
        .and_then(path::from_user_str);

    let mut cmd = Command::new(&shell_definition.command[0]);

    cmd.args(&shell_definition.command[1..])
        .envs(&shell_definition.environment);

    if let Some(path) = working_dir {
        cmd.current_dir(path);
    }
    cmd.spawn()
        .with_context(|| {
            format!("Unable to start shell for '{}'", shell_definition.service)
        })?
        .wait()?;

    Ok(())
}
