use crate::client::config::read_module_definitions;
use crate::client::module::InnerDefinition;
use crate::client::process::run_shell;
use crate::client::{cli::ClientConfig, module::shell_for_service};
use anyhow::{anyhow, bail, Result};

pub fn open_shell(service_name: &str, cfg: &ClientConfig) -> Result<()> {
    let module_defs = read_module_definitions(&cfg)?;
    let module_def =
        shell_for_service(service_name, &module_defs).ok_or_else(|| {
            anyhow!("Failed to find shell for service '{}'", service_name)
        })?;

    if let InnerDefinition::Shell(shell) = &module_def.inner {
        run_shell(shell)
    } else {
        bail!("Module provided is a {}, not a shell", module_def.kind)
    }
}
