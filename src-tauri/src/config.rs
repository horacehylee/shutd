use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// App configuration, stored at ~/.config/shutd/config.yaml.
/// Field names match the legacy ~/.shutd.yaml keys so old files migrate as-is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    #[serde(alias = "starttime")]
    pub start_time: String,
    #[serde(alias = "snoozeinterval")]
    pub snooze_interval: u32,
    pub notification: Notification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Notification {
    pub before: u32,
    pub duration: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            start_time: "01:00".into(),
            snooze_interval: 15,
            notification: Notification {
                before: 10,
                duration: 10,
            },
        }
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self {
            before: 10,
            duration: 10,
        }
    }
}

pub fn dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .expect("home directory is not set");
    home.join(".config").join("shutd")
}

pub fn config_path() -> PathBuf {
    dir().join("config.yaml")
}

pub fn log_path() -> PathBuf {
    dir().join("shutd.log")
}

/// Creates the config directory and migrates the legacy ~/.shutd.yaml on
/// first run. Returns the loaded config, writing defaults when no file exists.
pub fn load_or_create() -> Result<Config, String> {
    let config_dir = dir();
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("failed to create {}: {e}", config_dir.display()))?;

    let path = config_path();
    if !path.exists() {
        let legacy = dir()
            .parent()
            .and_then(|p| p.parent())
            .map(|home| home.join(".shutd.yaml"));
        if let Some(legacy) = legacy.filter(|p| p.exists()) {
            migrate(&legacy, &path)?;
        }
    }

    if path.exists() {
        parse(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
    } else {
        let config = Config::default();
        save(&config)?;
        Ok(config)
    }
}

fn migrate(legacy: &Path, target: &Path) -> Result<(), String> {
    let contents = fs::read_to_string(legacy).map_err(|e| e.to_string())?;
    parse(&contents)?; // validate before copying
    fs::write(target, contents).map_err(|e| format!("failed to migrate config: {e}"))
}

pub fn parse(yaml: &str) -> Result<Config, String> {
    if yaml.trim().is_empty() {
        return Ok(Config::default());
    }
    serde_yaml::from_str(yaml).map_err(|e| format!("invalid config: {e}"))
}

pub fn save(config: &Config) -> Result<(), String> {
    let yaml = serde_yaml::to_string(config).map_err(|e| e.to_string())?;
    fs::write(config_path(), yaml).map_err(|e| format!("failed to save config: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_keys() {
        let yaml = "startTime: \"02:30\"\nsnoozeInterval: 5\nnotification:\n  before: 3\n  duration: 7\n";
        let config = parse(yaml).unwrap();
        assert_eq!(
            config,
            Config {
                start_time: "02:30".into(),
                snooze_interval: 5,
                notification: Notification {
                    before: 3,
                    duration: 7
                }
            }
        );
    }

    #[test]
    fn parses_viper_lowercase_keys() {
        let yaml = "notification:\n  before: 10\n  duration: 10\nsnoozeinterval: 15\nstarttime: \"00:30\"\n";
        let config = parse(yaml).unwrap();
        assert_eq!(config.start_time, "00:30");
        assert_eq!(config.snooze_interval, 15);
    }

    #[test]
    fn empty_file_falls_back_to_defaults() {
        assert_eq!(parse("").unwrap(), Config::default());
    }

    #[test]
    fn rejects_bad_values() {
        assert!(parse("startTime: [1, 2]").is_err());
        assert!(parse("snoozeInterval: -5").is_err());
    }

    #[test]
    fn round_trips_through_yaml() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("startTime:"));
        assert_eq!(parse(&yaml).unwrap(), config);
    }
}
