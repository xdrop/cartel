use crate::client::module::{InnerDefinition, ModuleDefinition};
use anyhow::{bail, Context, Result};
use std::collections::HashSet;

pub fn non_existant_modules<'a>(
    module_names: &HashSet<&str>,
    to_validate: &[&'a str],
) -> Vec<&'a str> {
    let mut non_existent = Vec::new();
    for name in to_validate {
        let exists = module_names.contains(name);
        if !exists {
            non_existent.push(*name)
        }
    }
    non_existent
}

pub fn validate_modules_selected(
    module_names: &HashSet<&str>,
    to_validate: &[&str],
) -> Result<()> {
    let non_existant = non_existant_modules(&module_names, &to_validate);
    if !non_existant.is_empty() {
        bail!("The following modules do not exist: {:?}", non_existant)
    }
    Ok(())
}

pub fn validate_modules_unique(modules: &[ModuleDefinition]) -> Result<()> {
    let mut seen = HashSet::new();
    for module in modules {
        if seen.contains(&module.name) {
            bail!("The following module already exists: '{}'", module.name);
        }
        seen.insert(&module.name);
    }
    Ok(())
}

pub fn validate_module_names_exist(
    modules: &HashSet<String>,
    names: &[String],
) -> Result<()> {
    let non_existant: Vec<_> = names
        .iter()
        .filter(|n| !modules.contains(n.as_str()))
        .collect();

    if !non_existant.is_empty() {
        bail!("The following modules do not exist: {:?}", non_existant);
    }
    Ok(())
}

pub fn validate_fields(modules: &[ModuleDefinition]) -> Result<()> {
    for module in modules {
        match &module.inner {
            InnerDefinition::Service(svc_or_task)
            | InnerDefinition::Task(svc_or_task) => {
                if svc_or_task.shell.is_some()
                    && !svc_or_task.command.is_empty()
                {
                    bail!(
                        "Cannot have both a shell and command definition \
                        for module {}",
                        svc_or_task.name
                    );
                } else if svc_or_task.shell.is_none()
                    && svc_or_task.command.is_empty()
                {
                    bail!(
                        "Module must define one of 'shell' or 'command' for \
                        {}",
                        svc_or_task.name
                    );
                }
            }
            InnerDefinition::Check(check) => {
                if check.shell.is_some() && !check.command.is_empty() {
                    bail!(
                        "Cannot have both a shell and command definition \
                        for check {}",
                        check.name
                    );
                } else if check.shell.is_none() && check.command.is_empty() {
                    bail!(
                        "Module must define one of 'shell' or 'command' for \
                        {}",
                        check.name
                    );
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn validate_dependencies_exist(modules: &[ModuleDefinition]) -> Result<()> {
    let module_names: HashSet<_> =
        modules.iter().map(|m| m.name.clone()).collect();
    for module in modules {
        match module.inner {
            InnerDefinition::Group(ref grp) => {
                validate_module_names_exist(&module_names, &grp.dependencies)
                    .with_context(|| {
                        format!(
                            "Failed resolving dependencies of group '{}'",
                            module.name
                        )
                    })?
            }
            InnerDefinition::Shell(ref shell) => {
                let service_name = vec![shell.service.clone()];
                validate_module_names_exist(&module_names, &service_name)
                    .with_context(|| {
                        format!(
                            "Failed resolving service of shell '{}'",
                            module.name
                        )
                    })?
            }
            InnerDefinition::Service(ref svc_or_task)
            | InnerDefinition::Task(ref svc_or_task) => {
                validate_module_names_exist(
                    &module_names,
                    &svc_or_task.dependencies,
                )
                .with_context(|| {
                    format!(
                        "Failed resolving dependencies of service/task '{}'",
                        module.name
                    )
                })?;
                validate_module_names_exist(
                    &module_names,
                    &svc_or_task.ordered_dependencies,
                )
                .with_context(|| {
                    format!(
                        "Failed resolving ordered_dependencies of service/task '{}'",
                        module.name
                    )
                })?;
                validate_module_names_exist(
                    &module_names,
                    &svc_or_task.post_up,
                )
                .with_context(|| {
                    format!(
                        "Failed resolving post_up dependencies of service/task '{}'",
                        module.name
                    )
                })?;
                validate_module_names_exist(&module_names, &svc_or_task.post)
                    .with_context(|| {
                    format!(
                        "Failed resolving post dependencies of service/task '{}'",
                        module.name
                    )
                })?;
            }
            _ => {}
        }
    }
    Ok(())
}
