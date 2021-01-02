use crate::client::module::{CheckDefinitionV1, ServiceOrTaskDefinitionV1};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

pub fn run_task(task_definition: &ServiceOrTaskDefinitionV1) -> Result<()> {
    let working_dir = task_definition
        .working_dir
        .as_ref()
        .map(|d| PathBuf::from(d));

    let mut cmd = Command::new(&task_definition.command[0]);

    cmd.args(&task_definition.command[1..])
        .envs(&task_definition.environment);

    if let Some(path) = working_dir {
        cmd.current_dir(path);
    }
    let task = cmd
        .spawn()
        .with_context(|| {
            format!("Unable to run task '{}'", task_definition.name)
        })?
        .wait()?;

    Ok(())
}

pub fn run_check(check_definition: &CheckDefinitionV1) -> Result<ExitStatus> {
    let working_dir = check_definition
        .working_dir
        .as_ref()
        .map(|d| PathBuf::from(d));

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