use crate::client::module::{
    BaseDefinitionV1, CheckDefinitionV1, ModuleKindV1,
    ServiceOrTaskDefinitionV1,
};
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
) -> Result<Vec<BaseDefinitionV1>, serde_yaml::Error> {
    let mut parsed = serde_yaml::from_str_multidoc(source)?;
    for m in parsed.iter_mut() {
        match m {
            #[rustfmt::skip]
            BaseDefinitionV1::Service(inner) => inner.kind = ModuleKindV1::Service,
            BaseDefinitionV1::Task(inner) => inner.kind = ModuleKindV1::Task,
            _ => {}
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

pub fn read_module_definitions(
) -> Result<(Vec<ServiceOrTaskDefinitionV1>, Vec<CheckDefinitionV1>)> {
    match locate_config() {
        Some(mut config_file) => {
            let mut buffer = String::new();
            config_file
                .read_to_string(&mut buffer)
                .with_context(|| "While reading config file")?;

            let mut module_defs: Vec<BaseDefinitionV1> =
                parse_from_yaml_str(&buffer)
                    .with_context(|| "Failed to read module definitions")?;

            let mut services_and_tasks = vec![];
            let mut checks = vec![];

            while !module_defs.is_empty() {
                let module = module_defs.pop().unwrap();
                match module {
                    #[rustfmt::skip]
                    BaseDefinitionV1::Service(def) => services_and_tasks.push(def),
                    BaseDefinitionV1::Task(def) => services_and_tasks.push(def),
                    BaseDefinitionV1::Check(def) => checks.push(def),
                }
            }

            validate_modules_unique(&services_and_tasks, &checks)?;
            Ok((services_and_tasks, checks))
        }
        None => bail!("Failed to find config file"),
    }
}
