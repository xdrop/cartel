use crate::client::module::{InnerDefinition, ModuleDefinition, ModuleKind};
use crate::client::validation::validate_modules_unique;
use anyhow::{bail, Context, Result};
use std::env;
use std::fs::File;
use std::io::Read;
use std::option::Option;
use std::path::Path;

/// Parse one or more modules from the given string.
///
/// Parses module definitions in YAML format from the given string. One or more
/// definitions may be provided by separating with three dashes (---).
///
/// # Arguments
/// * `source` - The source string to parse from. It may contain
/// one or more modules separated by '---'
pub fn parse_from_yaml_str(
    source: &str,
) -> Result<Vec<ModuleDefinition>, serde_yaml::Error> {
    let mut parsed: Vec<ModuleDefinition> =
        serde_yaml::from_str_multidoc(source)?;
    for mut m in parsed.iter_mut() {
        match &mut m.inner {
            InnerDefinition::Service(def) => {
                m.kind = ModuleKind::Service;
                def.name = m.name.clone();
            }
            InnerDefinition::Task(def) => {
                m.kind = ModuleKind::Task;
                def.name = m.name.clone();
            }
            InnerDefinition::Check(def) => {
                m.kind = ModuleKind::Check;
                def.name = m.name.clone();
            }
            InnerDefinition::Group(def) => {
                m.kind = ModuleKind::Group;
                def.name = m.name.clone();
            }
        }
    }
    Ok(parsed)
}

/// Attempts to locate the config file in the given directory.
///
/// Tries to read the config file by looking in the current working directory.
/// Returns `Some(File)` if it has been found, or `None` otherwise.
pub fn locate_config() -> Option<File> {
    let cwd =
        env::current_dir().expect("Failed to get current working directory");
    let path_to_try = cwd.join(Path::new("./cartel.yaml"));
    let config_file = path_to_try.as_path();

    if config_file.exists() {
        return Some(
            File::open(config_file)
                .expect("Failed to open config file for reading"),
        );
    }
    None
}

pub fn read_module_definitions() -> Result<Vec<ModuleDefinition>> {
    match locate_config() {
        Some(mut config_file) => {
            let mut buffer = String::new();
            config_file
                .read_to_string(&mut buffer)
                .with_context(|| "While reading config file")?;

            let module_defs: Vec<ModuleDefinition> =
                parse_from_yaml_str(&buffer)
                    .with_context(|| "Failed to read module definitions")?;

            validate_modules_unique(&module_defs)?;
            Ok(module_defs)
        }
        None => bail!("Failed to find config file"),
    }
}
