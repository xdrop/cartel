use crate::client::cli::CliOptions;
use crate::client::config::read_module_definitions;
use crate::client::module::{module_by_name, ModuleKindV1};
use anyhow::{anyhow, bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn run_task_cmd(task_name: &str, cli_config: &CliOptions) -> Result<()> {
    let module_defs = read_module_definitions()?;
    let task_definition =
        module_by_name(task_name, &module_defs).ok_or_else(|| {
            anyhow!("Failed to find task with name '{}'", task_name)
        })?;

    if task_definition.kind != ModuleKindV1::Task {
        bail!("Expected task found: {}", task_definition.kind)
    }

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

    let mut task = cmd
        .spawn()
        .with_context(|| format!("Unable to run task '{}'", task_name))?
        .wait()?;
    Ok(())
}
