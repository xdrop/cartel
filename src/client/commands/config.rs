use crate::config;
use anyhow::Result;

pub fn set_option(key: &str, value: &str) -> Result<()> {
    let mut editable_cfg = config::read_persisted_config_as_editable()?;
    editable_cfg.set_option(key, value.to_owned())?;
    editable_cfg.save()?;
    tprint!("{} => {}", cbold!(key), cbold!(value));
    Ok(())
}

pub fn toggle_option(key: &str) -> Result<()> {
    let mut editable_cfg = config::read_persisted_config_as_editable()?;
    let new_value = editable_cfg.toggle_option(key)?;
    editable_cfg.save()?;
    tprint!("{} => {}", cbold!(key), cbold!(new_value));
    Ok(())
}

pub fn remove_option(key: &str) -> Result<()> {
    let mut editable_cfg = config::read_persisted_config_as_editable()?;
    editable_cfg.remove_option(key)?;
    editable_cfg.save()?;
    tprint!("{} => {}", cbold!(key), cbold!("None"));
    Ok(())
}

pub fn get_option(key: &str) -> Result<()> {
    let editable_cfg = config::read_persisted_config_as_editable()?;
    let value = editable_cfg.get_option(key)?;
    match value {
        Some(v) => tprint!("{} => {}", cbold!(key), cbold!(v)),
        None => tprint!("{} => {}", cbold!(key), cbold!("None")),
    }
    Ok(())
}

pub fn view_all_options() -> Result<()> {
    let editable_cfg = config::read_persisted_config_as_editable()?;
    for (key, val) in editable_cfg.get_all_options()? {
        tprint!(
            "{} => {}",
            cbold!(key),
            match val {
                Some(v) => cbold!(v),
                None => cbold!(String::from("None")),
            }
        )
    }
    Ok(())
}
