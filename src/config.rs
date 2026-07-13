use directories::BaseDirs;
use mlua::{Lua, Result};
use std::{
    cell::RefCell,
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

// Config struct that holds settings that are used throughout Mist
// We create default settings for each field then run the user defined
// lua script to overwrite the default settings
pub struct Config {
    theme: String,
    keys: Vec<KeyAction>,
}

impl Config {
    pub fn default() -> Config {
        Config {
            theme: String::from("nord"),
            keys: Vec::new(),
        }
    }
}

pub enum KeyAction {
    Add((String, String, String)),
    Remove((String, String)),
}

fn config_file_path() -> Result<PathBuf> {
    let config_dir = BaseDirs::new()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "No home directory"))?
        .config_dir()
        .join("mist")
        .join("init.lua");

    if let Some(dir) = config_dir.parent() {
        fs::create_dir_all(dir)?;
    };

    if !config_dir.exists() {
        fs::write(&config_dir, "-- Mist config file")?;
    };

    Ok(config_dir)
}

pub fn load_config() -> Result<Config> {
    let lua = Lua::new();

    let config = RefCell::new(Config::default());
    let config_dir = config_file_path()?;

    lua.scope(|scope| {
        let mist = lua.create_table()?;

        let theme = lua.create_table()?;
        theme.set(
            "set",
            scope.create_function(|_, name: String| {
                config.borrow_mut().theme = name;
                Ok(())
            })?,
        )?;
        mist.set("theme", theme)?;

        let keys = lua.create_table()?;
        keys.set(
            "set",
            scope.create_function(|_, (mode, key_set, command): (String, String, String)| {
                config
                    .borrow_mut()
                    .keys
                    .push(KeyAction::Add((mode, key_set, command)));
                Ok(())
            })?,
        )?;
        keys.set(
            "unset",
            scope.create_function(|_, (mode, key_set): (String, String)| {
                config
                    .borrow_mut()
                    .keys
                    .push(KeyAction::Remove((mode, key_set)));
                Ok(())
            })?,
        )?;
        mist.set("keys", keys)?;

        lua.globals().set("mist", mist)?;

        lua.load(config_dir).exec()?;

        Ok(())
    })?;

    Ok(config.into_inner())
}
