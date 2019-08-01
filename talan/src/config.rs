use crate::task::Task;
use failure::Error;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// This module handles all configuration management for Talan.
#[derive(PartialEq, Debug, Serialize, Default, Deserialize)]
pub struct Options {
    pub reload_tasks: bool,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    // Stored as i32 because imgui doesn't bind to unsigned ints.
    pub gear: [i32; xiv::JOB_CNT],
    pub options: Options,
    pub tasks: Vec<Task>,
}

const CONFIG_PATH: &str = "config.json";

// Reads the config from disk. Returns a default config
// if one cannot be found.
pub fn read_config() -> Config {
    read_config_internal(Path::new(CONFIG_PATH))
}

// Writes |cfg| to disk.
pub fn write_config(cfg: &Config) -> Result<(), Error> {
    write_config_internal(cfg, Path::new(CONFIG_PATH))
}

fn read_config_internal(path: &Path) -> Config {
    match read_config_from_file(path) {
        Ok(c) => c,
        Err(_) => {
            println!("Config not found, creating a new one!");
            Config::default()
        }
    }
}

fn read_config_from_file(path: &Path) -> Result<Config, Error> {
    let mut contents = String::new();
    let mut f = File::open(path)?;
    f.read_to_string(&mut contents)?;
    Ok(serde_json::from_str(&contents)?)
}

fn write_config_internal(cfg: &Config, path: &Path) -> Result<(), Error> {
    let mut f = File::create(path)?;
    f.write_all(serde_json::to_string(&cfg)?.as_bytes())
        .unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_and_read() -> Result<(), Error> {
        let file = NamedTempFile::new()?;
        let mut c1 = Config::default();
        c1.gear[0] = 1;
        c1.gear[1] = 2;
        c1.gear[2] = 3;
        c1.gear[3] = 4;
        c1.gear[4] = 5;
        c1.gear[5] = 6;
        c1.gear[6] = 7;
        c1.gear[7] = 8;
        assert!(write_config_internal(&c1, file.path()).is_ok());
        let c2 = read_config_internal(file.path());
        assert_eq!(c1, c2);
        Ok(())
    }

    #[test]
    fn test_default_config() -> Result<(), Error> {
        let file = NamedTempFile::new()?;
        assert_eq!(read_config_internal(file.path()), Config::default());
        Ok(())
    }
}
