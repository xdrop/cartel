use crate::client::module::{
    Healthcheck, InnerDefinition, ModuleDefinition, ModuleKind,
};
use crate::client::validation::validate_modules_unique;
use crate::path;
use anyhow::{bail, Context, Result};
use std::env;
use std::fs::File;
use std::io::Read;
use std::option::Option;
use std::path::{Path, PathBuf};

/// Parse one or more modules from the given string.
///
/// Parses module definitions in YAML format from the given string. One or more
/// definitions may be provided by separating with three dashes (---).
///
/// # Arguments
/// * `source` - The source string to parse from. It may contain
/// one or more modules separated by '---'
/// * `path` - The path to the *directory* of the module definitions file.
pub fn parse_from_yaml_str(
    source: &str,
    path: &Path,
) -> Result<Vec<ModuleDefinition>> {
    let mut parsed: Vec<ModuleDefinition> =
        serde_yaml::from_str_multidoc(source)?;
    for mut m in parsed.iter_mut() {
        match &mut m.inner {
            InnerDefinition::Service(ref mut def) => {
                m.kind = ModuleKind::Service;
                def.name = m.name.clone();
                update_path(&mut def.working_dir, path)?;
                if let Some(Healthcheck::Exec(ref mut exec)) = def.healthcheck {
                    update_path(&mut exec.working_dir, path)?;
                }
            }
            InnerDefinition::Task(def) => {
                m.kind = ModuleKind::Task;
                def.name = m.name.clone();
                update_path(&mut def.working_dir, path)?;
                if let Some(Healthcheck::Exec(ref mut exec)) = def.healthcheck {
                    update_path(&mut exec.working_dir, path)?;
                }
            }
            InnerDefinition::Check(def) => {
                m.kind = ModuleKind::Check;
                def.name = m.name.clone();
                update_path(&mut def.working_dir, path)?;
            }
            InnerDefinition::Group(def) => {
                m.kind = ModuleKind::Group;
                def.name = m.name.clone();
            }
        }
    }
    Ok(parsed)
}

/// Canonicalize the path in the given option.
///
/// The incoming option's content is replaced by a new [String] containing the
/// canonicalized version of the path represented by the [String].
///
/// Paths like `~/mypath` or `./../mypath/..` will be converted to absolute
/// paths, while also resolving any symlinks.
///
/// # Errors
///
/// If the path provided cannot be canonicalized then an error will be returned.
fn update_path(o: &mut Option<String>, relative_to: &Path) -> Result<()> {
    if let Some(path) = o.as_mut() {
        let canon = path::canonicalize_str(path.as_str(), relative_to)
            .with_context(|| {
                format!("Failed to parse path: {}", path.as_str())
            })?;
        *path = canon;
    }
    Ok(())
}

/// Attempts to locate the config file in the given directory.
///
/// Tries to read the config file by looking in the current working directory.
/// Returns `Some((File, PathBuf))` if it has been found, or `None` otherwise.
pub fn locate_config() -> Option<(File, PathBuf)> {
    let cwd =
        env::current_dir().expect("Failed to get current working directory");
    let path_to_try = cwd.join(Path::new("./cartel.yaml"));
    let config_file = path_to_try.as_path();

    if config_file.exists() {
        return Some((
            File::open(config_file)
                .expect("Failed to open config file for reading"),
            path_to_try,
        ));
    }
    None
}

pub fn read_module_definitions() -> Result<Vec<ModuleDefinition>> {
    match locate_config() {
        Some((mut config_file, path)) => {
            let mut buffer = String::new();
            config_file
                .read_to_string(&mut buffer)
                .with_context(|| "While reading config file")?;

            let module_defs: Vec<ModuleDefinition> =
                parse_from_yaml_str(&buffer, path.as_path().parent().unwrap())
                    .with_context(|| "Failed to read module definitions")?;

            validate_modules_unique(&module_defs)?;
            Ok(module_defs)
        }
        None => bail!("Failed to find config file"),
    }
}
