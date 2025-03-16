use crate::client::cli::ClientConfig;
use crate::client::cmd::cmd_in_shell;
use crate::client::definitions::get_module_by_name;
use crate::client::module::InnerDefinition;
use crate::command_builder::CommandBuilder;
use anyhow::{bail, Result};
use std::os::unix::prelude::CommandExt;
use std::path::Path;

pub fn exec_cmd(
    service: &str,
    command: &[&str],
    cfg: &ClientConfig,
) -> Result<()> {
    let module = get_module_by_name(service, cfg)?;

    let working_dir = if let Some(ref m) = module {
        if let InnerDefinition::Service(svc) = &m.inner {
            &svc.working_dir
        } else {
            bail!(
                "Expected service with name {} but found {:?}",
                service,
                m.kind
            );
        }
    } else {
        bail!("Service with name {} not found", service);
    };

    let cmd_line = cmd_in_shell(command);

    let mut cmd = CommandBuilder::new(&cmd_line);
    cmd.work_dir(working_dir.as_ref().map(Path::new));

    #[cfg(unix)]
    {
        let _ = cmd.build().exec(); // The process ends here
    }

    #[cfg(windows)]
    {
        cmd.build().spawn()?.wait()?;
    }

    Ok(())
}
