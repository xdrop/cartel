use crate::client::cli::ClientConfig;
use crate::client::module::{
    InnerDefinition, ModuleDefinition, ModuleKind, Probe,
};
use crate::client::validation::{
    validate_dependencies_exist, validate_fields, validate_modules_unique,
};
use crate::path;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;
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
    let mut parsed: Vec<ModuleDefinition> = vec![];
    for (idx, document) in
        serde_yaml::Deserializer::from_str(source).enumerate()
    {
        let value = Value::deserialize(document)?;

        // Attempt to retrieve and clone the name in an attempt to provide a
        // useful error message to the user.
        let mod_name =
            value.get("name").and_then(|v| v.as_str()).map(String::from);
        let mod_kind =
            value.get("kind").and_then(|v| v.as_str()).map(String::from);

        let module: ModuleDefinition = serde_yaml::from_value(value)
            .with_context(|| {
                if let Some(name) = mod_name {
                    format!(
                        "Couldn't parse {} definition with name: {}",
                        mod_kind.unwrap_or("module".to_string()).to_lowercase(),
                        name
                    )
                } else {
                    format!("Couldn't parse module at position: {}", idx)
                }
            })?;

        parsed.push(module);
    }
    for m in parsed.iter_mut() {
        match &mut m.inner {
            InnerDefinition::Service(ref mut def) => {
                m.kind = ModuleKind::Service;
                def.name = m.name.clone();
                update_path(&mut def.working_dir, path)?;
                if let Some(Probe::Exec(ref mut exec)) = def.readiness_probe {
                    update_path(&mut exec.working_dir, path)?;
                }
            }
            InnerDefinition::Task(def) => {
                m.kind = ModuleKind::Task;
                def.name = m.name.clone();
                update_path(&mut def.working_dir, path)?;
                if let Some(Probe::Exec(ref mut exec)) = def.readiness_probe {
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
            InnerDefinition::Shell(def) => {
                m.kind = ModuleKind::Shell;
                update_path(&mut def.working_dir, path)?;
                def.name = format!("{}-service-shell", def.service);
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
    } else {
        o.get_or_insert_with(|| {
            relative_to
                .to_str()
                .expect("Failed to convert path to str in update_path")
                .to_string()
        });
    }
    Ok(())
}

/// Scans for the given file in the directory tree.
///
/// Tries to discover `file_to_try` in the current directory or any of it's
/// ancestor directories.
///
/// Navigates the directory tree starting at `current_path` and walking
/// upwards for each failure. The search will stop once a path component
/// with no parent is encountered.
fn scan_directories_for(
    current_path: &Path,
    file_to_try: &str,
) -> Option<PathBuf> {
    let mut buf = current_path.to_path_buf();
    let mut found = None;
    loop {
        buf.push(file_to_try);

        if buf.exists() {
            found = Some(buf);
            break;
        }

        buf.pop(); // pop file
        let has_parent = buf.pop(); // pop dir

        if !has_parent {
            break;
        }
    }
    found
}

/// Try to locate file in the given directory.
fn try_locate_in_dir(dir: &str, file_name: &str) -> Option<PathBuf> {
    let path = path::from_user_str(dir);
    if let Some(mut path) = path {
        path.push(file_name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Try to locate from the given file path.
fn try_locate_from_file_path(file_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(file_path);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Attempts to locate the module definitions file.
///
/// Tries to locate the module definition file checking each parent directory,
/// in order. Returns the file and it's path if found, or returns an error
/// otherwise.
pub fn locate_module_definitions_file(
    provided: &Option<String>,
    default_dir: &Option<String>,
) -> Option<PathBuf> {
    if let Some(provided) = provided {
        return try_locate_from_file_path(provided);
    };

    let cwd =
        env::current_dir().expect("Failed to get current working directory");
    let scan_result = scan_directories_for(cwd.as_path(), "cartel.yml");

    if scan_result.is_some() {
        scan_result
    } else if let Some(default_dir) = default_dir {
        try_locate_in_dir(default_dir, "cartel.yml")
    } else {
        None
    }
}

/// Open the given module definitions file.
///
/// If no file is given, an attempt to locate the file is done instead.
pub fn open_module_file(
    file: &Option<String>,
    default_dir: &Option<String>,
) -> Result<(File, PathBuf)> {
    let module_file = locate_module_definitions_file(file, default_dir);

    if let Some(module_file) = module_file {
        return Ok((
            File::open(module_file.as_path())
                .context("Failed to open file for reading")?,
            module_file,
        ));
    }

    bail!("Failed to locate module definitions file (cartel.yml)")
}

/// Try to find a file with the given name next to file pointed by `path`.
fn try_find_sibling(path: &Path, file_name: &str) -> Option<PathBuf> {
    let mut buf = path.to_path_buf();

    // Pop file name from sibling file path to get dir
    buf.pop();
    buf.push(file_name);

    if buf.exists() {
        Some(buf)
    } else {
        None
    }
}

/// Attempt to locate a module definition overrides file.
fn locate_override_file(
    module_definitions_file: &Path,
    cfg: &ClientConfig,
) -> Option<PathBuf> {
    if let Some(file_path) = &cfg.override_file {
        let path = Path::new(&file_path);
        if path.exists() {
            Some(path.to_path_buf())
        } else {
            None
        }
    } else {
        try_find_sibling(module_definitions_file, "cartel.override.yml")
    }
}

/// Open the given override module definitions file.
///
/// If no file is given, an attempt to locate the file is done instead.
fn open_override_file(
    mod_def_file: &Path,
    cfg: &ClientConfig,
) -> Result<Option<(File, PathBuf)>> {
    let override_file = locate_override_file(mod_def_file, cfg);
    if let Some(file_path) = override_file {
        let file = File::open(file_path.as_path())
            .context("Failed to open override file path")?;
        Ok(Some((file, file_path)))
    } else {
        Ok(None)
    }
}

/// Parse a module definition file.
fn parse_module_def_file(
    mut file: File,
    path: &Path,
) -> Result<Vec<ModuleDefinition>> {
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .with_context(|| "While reading module definition file")?;

    let canonicalized = path.canonicalize()?;

    let module_defs: Vec<ModuleDefinition> =
        parse_from_yaml_str(&buffer, canonicalized.parent().unwrap())
            .with_context(|| "Failed to read module definitions")?;

    validate_modules_unique(&module_defs)?;
    validate_dependencies_exist(&module_defs)?;
    validate_fields(&module_defs)?;

    Ok(module_defs)
}

/// Merge main module definitions with overrides.
///
/// In the case of a name clash the override file takes priority when merging.
/// All other definitions are kept from both files.
fn merge_module_definitions(
    main: Vec<ModuleDefinition>,
    mut overriden: Vec<ModuleDefinition>,
) -> Vec<ModuleDefinition> {
    let mut merged = main;
    let mut module_map = HashMap::new();

    for (idx, m) in merged.iter().enumerate() {
        module_map.insert(m.name.clone(), idx);
    }

    while let Some(m) = overriden.pop() {
        if let Some(idx) = module_map.get(&m.name) {
            merged[*idx] = m;
        } else {
            merged.push(m);
        }
    }
    merged
}

/// Read module definitions from the filesystem.
///
/// Reads module definitions by attempting to locate a module definitions file
/// as well as any potential overrides files. If an override file is found it is
/// merged with the main module definitions file.
///
/// The search for the module definitions file begins at the current directory,
/// and walks upwards until a file is found. In case of a file not located then
/// the default directory from the client config is used.
pub fn read_module_definitions(
    cfg: &ClientConfig,
) -> Result<Vec<ModuleDefinition>> {
    let (mod_def_file, path) =
        open_module_file(&cfg.module_file, &cfg.default_dir)?;

    let module_defs = parse_module_def_file(mod_def_file, path.as_path())?;

    if let Some((override_file, override_file_path)) =
        open_override_file(path.as_path(), cfg)?
    {
        let override_module_defs =
            parse_module_def_file(override_file, override_file_path.as_path())
                .context("Failed while parsing overrides file")?;
        return Ok(merge_module_definitions(module_defs, override_module_defs));
    }

    Ok(module_defs)
}

/// Retrieves a module definition by name.
///
/// This causes a full module definitions parse so prefer calling
/// [`read_module_definitions`] directly if more than one module is required to
/// be retrieved.
///
/// This function will error if no such module can be found.
pub fn get_module_by_name(
    name: &str,
    cfg: &ClientConfig,
) -> Result<Option<ModuleDefinition>> {
    let module_defs = read_module_definitions(cfg)?;
    let found = module_defs.into_iter().find(|m| m.name == name);
    match found {
        Some(module_def) => Ok(Some(module_def)),
        None => Ok(None),
    }
}
