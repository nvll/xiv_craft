use crate::task::Task;
use failure::Error;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Options {
    // Stored as i32 because imgui doesn't bind to unsigned ints.
    #[serde(default)]
    pub gear: [i32; xiv::JOB_CNT],
    #[serde(default)]
    pub specialist: [bool; xiv::JOB_CNT],
    #[serde(default)]
    pub non_doh_gear: i32,
    #[serde(default)]
    pub reload_tasks: bool,
    #[serde(default)]
    pub use_slow_navigation: bool,
}

// Placeholder.
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Macro {}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub options: Options,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub macros: Vec<Macro>,
}

pub const DEFAULT_CONFIG_FILE: &str = "config.json";

pub fn get_config(path: Option<&Path>) -> Config {
    let p = path.unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE));
    read_config(p).unwrap_or_else(|e| {
        log::info!(
            "Unable to open {} ({}), creating a new default config.",
            p.display(),
            e.to_string()
        );
        Config::default()
    })
}

// Reads the config from disk. Returns a default config if one cannot be found.
pub fn read_config(path: &Path) -> Result<Config, Error> {
    Ok(serde_json::from_str::<Config>(&std::fs::read_to_string(
        path,
    )?)?)
}

// Writes |cfg| to disk.
pub fn write_config(path: Option<&Path>, cfg: &Config) -> Result<(), Error> {
    let p = path.unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE));
    std::fs::write(p, serde_json::to_string_pretty(&cfg)?.as_bytes())?;
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
        c1.options.gear[0] = 1;
        c1.options.gear[1] = 2;
        c1.options.gear[2] = 3;
        c1.options.gear[3] = 4;
        c1.options.gear[4] = 5;
        c1.options.gear[5] = 6;
        c1.options.gear[6] = 7;
        c1.options.gear[7] = 8;
        assert!(write_config(Some(file.path()), &c1).is_ok());
        let c2 = read_config(file.path())?;
        assert_eq!(c1, c2);
        Ok(())
    }

    #[test]
    fn test_default_config() -> Result<(), Error> {
        assert_eq!(get_config(None), Config::default());
        Ok(())
    }
}
