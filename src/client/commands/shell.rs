use crate::client::cli::ClientConfig;
use crate::client::definitions::read_module_definitions;
use crate::client::module::{shell_for_service, InnerDefinition};
use crate::client::process::run_shell;
use anyhow::{anyhow, bail, Result};

pub fn open_shell(
    service_name: &str,
    shell_type: Option<&str>,
    cfg: &ClientConfig,
) -> Result<()> {
    let module_defs = read_module_definitions(cfg)?;
    let module_def = shell_for_service(service_name, shell_type, &module_defs)
        .ok_or_else(|| {
            anyhow!("Failed to find shell for service '{}'", service_name)
        })?;

    if let InnerDefinition::Shell(shell) = &module_def.inner {
        run_shell(shell)
    } else {
        bail!("Module provided is a {}, not a shell", module_def.kind)
    }
}
