use crate::constants::PROJECT_DIR;
use anyhow::{bail, Context, Result};
use phf::phf_map;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use toml_edit::{value, Document, Item, Value};

#[derive(Deserialize)]
pub struct DaemonConfig {
    /// The port to reach the daemon at.
    pub port: Option<String>,
    /// Turn on the experimental env grabber.
    #[serde(deserialize_with = "bool_from_enabled_disabled")]
    pub use_env_grabber: Option<bool>,
}

fn bool_from_enabled_disabled<'de, D>(
    deserializer: D,
) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let deserialized = String::deserialize(deserializer)?;
    let result = if deserialized == "enabled" {
        true
    } else if deserialized == "disabled" {
        false
    } else {
        return Err(Error::invalid_value(
            Unexpected::Str(&deserialized),
            &"\"enabled\" or \"disabled\"",
        ));
    };
    Ok(Some(result))
}

#[derive(Deserialize)]
pub struct ClientConfig {
    /// The default directory to use when searching for module definitions.
    pub default_dir: Option<String>,
}

#[derive(Deserialize)]
pub struct PersistedConfig {
    pub client: ClientConfig,
    pub daemon: DaemonConfig,
}

fn default_config_file_path() -> PathBuf {
    let mut config_path =
        dirs::home_dir().expect("Failed to locate users home dir");
    config_path.push(PROJECT_DIR);
    config_path.push("config.toml");
    config_path
}

pub fn create_config_if_not_exists() -> Result<()> {
    let path = default_config_file_path();
    let file = OpenOptions::new().write(true).create_new(true).open(path);
    match file {
        Ok(mut created) => {
            let content = "[daemon]\n\n[client]";
            created
                .write_all(content.as_bytes())
                .with_context(|| "Failed to write empty config file")?;
            Ok(())
        }
        Err(e) => {
            // Already exists if fine, any other error is not
            if e.kind() != io::ErrorKind::AlreadyExists {
                return Err(e.into());
            }
            Ok(())
        }
    }
}

/// Reads the persisted configuration from the given path.
///
/// The file is expected to be of TOML format and conform to the schema. An
/// error will be returned if the file does not exist.
pub fn read_persisted_config_from_path(path: &Path) -> Result<PersistedConfig> {
    let toml_content = fs::read_to_string(path)
        .with_context(|| "Failed to read config file")?;
    let document = toml::from_str::<PersistedConfig>(&toml_content)?;
    Ok(document)
}

/// Reads the persisted configuration from the default location.
///
/// The default location is defined in src/constants.rs as `PROJECT_DIR`.
///
/// The file is expected to be of TOML format and conform to the schema. An
/// error will be returned if the file does not exist.
pub fn read_persisted_config() -> Result<PersistedConfig> {
    let default_path = default_config_file_path();
    let config = read_persisted_config_from_path(&default_path)?;
    Ok(config)
}

/// Provides an editing interface to the configuration file in the given path.
///
/// The file is expected to be of TOML format and conform to the schema. If the
/// file does not exist, an error will be returned.
pub fn read_persisted_config_as_editable_from_path(
    path: &Path,
) -> Result<EditableConfig> {
    let toml_content = fs::read_to_string(path)
        .with_context(|| "Failed to read config file")?;
    let document = toml_content
        .parse::<Document>()
        .with_context(|| "Failed to parse TOML config file")?;
    Ok(EditableConfig::from(document, path))
}

/// Provides an editing interface to the configuration file in the default path.
///
/// The file is expected to be of TOML format and conform to the schema. If the
/// file does not exist, it will be created.
pub fn read_persisted_config_as_editable() -> Result<EditableConfig> {
    let default_path = default_config_file_path();
    read_persisted_config_as_editable_from_path(&default_path)
}

pub struct EditableConfig {
    raw_toml: Document,
    path: PathBuf,
}

static KEY_TO_PATH: phf::Map<&'static str, [&'static str; 2]> = phf_map! {
    "daemon.port" => ["daemon", "port"],
    "daemon.use_env_grabber" => ["daemon", "use_env_grabber"],
    "client.default_dir" => ["client", "default_dir"],
};

impl EditableConfig {
    fn from(document: Document, path: &Path) -> Self {
        Self {
            raw_toml: document,
            path: path.to_owned(),
        }
    }

    fn get_option_path(key: &str) -> Result<(&'static str, &'static str)> {
        if !KEY_TO_PATH.contains_key(key) {
            bail!("Unsupported setting '{}'", key);
        }
        let [namespace, key] = KEY_TO_PATH.get(key).unwrap();
        Ok((namespace, key))
    }

    pub fn get_option(&self, key: &str) -> Result<Option<String>> {
        let (namespace, key) = Self::get_option_path(key)?;
        match &self.raw_toml[namespace][key] {
            Item::None => Ok(None),
            Item::Value(Value::String(s)) => Ok(Some(s.value().to_string())),
            _ => bail!("Unsupported format for key '{}'. All options should be strings.", key),
        }
    }

    pub fn set_option(&mut self, key: &str, new_val: String) -> Result<()> {
        let (namespace, key) = Self::get_option_path(key)?;
        self.raw_toml[namespace][key] = value(new_val);
        Ok(())
    }

    pub fn remove_option(&mut self, key: &str) -> Result<()> {
        let (namespace, key) = Self::get_option_path(key)?;
        self.raw_toml[namespace][key] = Item::None;
        Ok(())
    }

    pub fn toggle_option(&mut self, key: &str) -> Result<String> {
        let (namespace, key) = Self::get_option_path(key)?;
        let new_value = match &self.raw_toml[namespace][key] {
            Item::None => "enabled",
            Item::Value(Value::String(s)) => {
                if s.value() == "enabled" {
                    "disabled"
                } else {
                    "enabled"
                }
            }
            _ => "enabled",
        };
        self.raw_toml[namespace][key] = value(new_value);
        Ok(new_value.to_string())
    }

    pub fn save(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(false)
            .truncate(true)
            .open(&self.path)
            .with_context(|| "While saving new TOML config")?;

        file.write_all(self.raw_toml.to_string().as_bytes())?;
        Ok(())
    }
}
