use crate::client::module::ModuleDefinitionV1;
use anyhow::{bail, Result};
use std::collections::HashSet;

pub fn non_existant_modules<'a>(
    module_names: &HashSet<&str>,
    to_validate: &Vec<&'a str>,
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
    to_validate: &Vec<&str>,
) -> Result<()> {
    let non_existant = non_existant_modules(&module_names, &to_validate);
    if non_existant.len() > 0 {
        bail!("The following modules do not exist: {:?}", non_existant)
    }
    Ok(())
}

pub fn validate_modules_unique(
    modules: &Vec<ModuleDefinitionV1>,
) -> Result<()> {
    let mut seen = HashSet::new();
    for module in modules {
        if seen.contains(&module.name) {
            bail!("The following module already exists: '{}'", module.name);
        }
        seen.insert(&module.name);
    }
    Ok(())
}
