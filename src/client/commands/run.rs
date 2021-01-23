use crate::client::cli::ClientConfig;
use crate::client::config::read_module_definitions;
use crate::client::module::{module_by_name, InnerDefinition};
use crate::client::process::run_task;
use anyhow::{anyhow, bail, Result};

pub fn run_task_cmd(task_name: &str, cfg: &ClientConfig) -> Result<()> {
    let module_defs = read_module_definitions(&cfg)?;
    let module_def =
        module_by_name(task_name, &module_defs).ok_or_else(|| {
            anyhow!("Failed to find task with name '{}'", task_name)
        })?;

    if let InnerDefinition::Task(task) = &module_def.inner {
        run_task(task)
    } else {
        bail!("Module provided is a {}, not a task", module_def.kind)
    }
}
