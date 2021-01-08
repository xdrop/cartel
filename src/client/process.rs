use crate::client::module::{CheckDefinition, ServiceOrTaskDefinition};
use crate::path;
use anyhow::{Context, Result};
use std::process::{Command, ExitStatus, Stdio};

pub fn run_task(task_definition: &ServiceOrTaskDefinition) -> Result<()> {
    let working_dir = task_definition
        .working_dir
        .as_deref()
        .and_then(path::from_user_str);

    let mut cmd = Command::new(&task_definition.command[0]);

    cmd.args(&task_definition.command[1..])
        .envs(&task_definition.environment);

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

    let mut cmd = Command::new(&check_definition.command[0]);

    cmd.args(&check_definition.command[1..])
        .stdout(Stdio::null());

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
