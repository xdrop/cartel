use crate::client::module::{
    CheckDefinition, ServiceOrTaskDefinition, ShellDefinition,
    SuggestedFixDefinition,
};
use crate::command_builder::CommandBuilder;
use crate::path;
use anyhow::{bail, Context, Result};
use std::process::ExitStatus;

pub fn run_task(task_definition: &ServiceOrTaskDefinition) -> Result<()> {
    let working_dir = task_definition
        .working_dir
        .as_deref()
        .and_then(path::from_user_str);

    let cmd_line = task_definition.cmd_line();
    let mut cmd = CommandBuilder::new(&cmd_line);

    cmd.env(&task_definition.environment)
        .work_dir(working_dir.as_deref());

    cmd.build()
        .spawn()
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
    let mut cmd = CommandBuilder::new(&cmd_line);

    cmd.stdout_null()
        .stderr_null()
        .work_dir(working_dir.as_deref());

    let check_result = cmd
        .build()
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

    let cmd_line = shell_definition.cmd_line();
    let mut cmd = CommandBuilder::new(&cmd_line);

    cmd.env(&shell_definition.environment)
        .work_dir(working_dir.as_deref());

    cmd.build()
        .spawn()
        .with_context(|| {
            format!("Unable to start shell for '{}'", shell_definition.service)
        })?
        .wait()?;

    Ok(())
}

pub fn apply_suggested_fix(
    suggested_fix_definition: &SuggestedFixDefinition,
) -> Result<()> {
    let working_dir = suggested_fix_definition
        .working_dir
        .as_deref()
        .and_then(path::from_user_str);

    let cmd_line = suggested_fix_definition.cmd_line();
    let mut cmd = CommandBuilder::new(&cmd_line);

    cmd.stdout_null()
        .stderr_null()
        .work_dir(working_dir.as_deref());

    let check_result = cmd.build().spawn()?.wait()?;
    if !check_result.success() {
        bail!("Suggested fix returned non-zero exit code");
    }

    Ok(())
}
