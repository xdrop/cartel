use crate::client::cli::CliOptions;
use crate::client::config::read_module_definitions;
use crate::client::module::{module_by_name, ModuleKindV1};
use crate::client::process::run_task;
use anyhow::{anyhow, bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn run_task_cmd(task_name: &str, cli_config: &CliOptions) -> Result<()> {
    let module_defs = read_module_definitions()?.0;
    let task_definition =
        module_by_name(task_name, &module_defs).ok_or_else(|| {
            anyhow!("Failed to find task with name '{}'", task_name)
        })?;

    if task_definition.kind != ModuleKindV1::Task {
        bail!("Expected task found: {}", task_definition.kind)
    }

    run_task(task_definition)
}
