use crate::client::cli::CliOptions;
use crate::client::module::{
    Healthcheck, InnerDefinition, ModuleDefinition, ModuleKind,
};
use crate::client::validation::validate_modules_unique;
use crate::path;
use anyhow::{bail, Context, Result};
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

/// Discover the given file in the directory tree.
///
/// Tries to discover `file_to_try` in the current directory or any of it's
/// ancestor directories.
///
/// Navigates the directory tree starting at `current_path` and walking
/// upwards for each failure. The search will stop once a path component
/// with no parent is encountered.
fn discover_file(current_path: &Path, file_to_try: &str) -> Option<PathBuf> {
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

/// Attempts to locate the module definitions file.
///
/// Tries to locate the module definition file checking each parent directory,
/// in order. Returns the file and it's path if found, or returns an error
/// otherwise.
pub fn try_locate_file() -> Result<(File, PathBuf)> {
    let cwd =
        env::current_dir().expect("Failed to get current working directory");
    let module_file = discover_file(cwd.as_path(), "cartel.yaml");

    if let Some(module_file) = module_file {
        if module_file.exists() {
            return Ok((File::open(module_file.as_path())?, module_file));
        }
    }
    bail!("Failed to locate cartel.yaml in current or ancestor directories")
}

/// Open the given module definitions file.
///
/// If no file is given, an attempt to locate the file is done instead.
pub fn open_module_file(file: &Option<String>) -> Result<(File, PathBuf)> {
    if let Some(path) = file {
        file_from_str_path(path.as_str())
    } else {
        try_locate_file()
    }
}

/// Open the given override module definitions file.
///
/// If no file is given, an attempt to locate the file is done instead.
fn open_override_file(
    mod_def_file: &Path,
    cli_config: &CliOptions,
) -> Result<Option<(File, PathBuf)>> {
    let override_file = locate_override_file(mod_def_file, cli_config);
    if let Some(file_path) = override_file {
        let file = File::open(file_path.as_path())
            .context("Failed to open override file path")?;
        Ok(Some((file, file_path)))
    } else {
        Ok(None)
    }
}

pub fn file_from_str_path(file_path: &str) -> Result<(File, PathBuf)> {
    let path = PathBuf::from(file_path);
    if path.exists() {
        return Ok((
            File::open(&path)
                .context("Failed to open given file for reading")?,
            path,
        ));
    }
    bail!("File at {} not found", file_path)
}

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

fn locate_override_file(
    module_definitions_file: &Path,
    cli_config: &CliOptions,
) -> Option<PathBuf> {
    if let Some(file_path) = &cli_config.override_file {
        let path = Path::new(&file_path);
        if path.exists() {
            Some(path.to_path_buf())
        } else {
            None
        }
    } else {
        try_find_sibling(module_definitions_file, "cartel.override.yaml")
    }
}

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

    Ok(module_defs)
}

fn merge_module_definitions(
    main: Vec<ModuleDefinition>,
    mut overriden: Vec<ModuleDefinition>,
) -> Vec<ModuleDefinition> {
    let mut merged = main;
    let mut module_map = HashMap::new();

    for (idx, m) in merged.iter().enumerate() {
        module_map.insert(m.name.clone(), idx);
    }

    while !overriden.is_empty() {
        let m = overriden.pop().unwrap();

        if let Some(idx) = module_map.get(&m.name) {
            merged[*idx] = m;
        } else {
            merged.push(m);
        }
    }
    merged
}

pub fn read_module_definitions(
    opts: &CliOptions,
) -> Result<Vec<ModuleDefinition>> {
    let (mod_def_file, path) = open_module_file(&opts.module_file)?;

    let module_defs = parse_module_def_file(mod_def_file, path.as_path())?;

    if let Some((override_file, override_file_path)) =
        open_override_file(path.as_path(), opts)?
    {
        let override_module_defs =
            parse_module_def_file(override_file, override_file_path.as_path())
                .context("Failed while parsing overrides file")?;
        return Ok(merge_module_definitions(module_defs, override_module_defs));
    }

    Ok(module_defs)
}
