use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

/// Times before this hour belong to the previous day's session and fire on
/// the next calendar date unless the user overrides `nextDay`.
const MORNING_BOUNDARY: &str = "06:00";

/// Per-weekday shutdown rule; index 0 is Monday.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WeekdayRule {
    pub time: String,
    pub next_day: bool,
}

impl Default for WeekdayRule {
    fn default() -> Self {
        Self {
            time: "01:00".into(),
            next_day: true,
        }
    }
}

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
    /// Always 7 entries after loading; empty means "derive from start_time".
    #[serde(default)]
    pub weekdays: Vec<WeekdayRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Notification {
    pub before: u32,
    pub duration: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self::with_start_time("01:00")
    }
}

impl Config {
    /// Daily schedule expressed as seven identical weekday slots; used for
    /// legacy configs and tests.
    pub fn with_start_time(start_time: &str) -> Self {
        Self {
            start_time: start_time.into(),
            snooze_interval: 15,
            notification: Notification::default(),
            weekdays: derive_weekdays(start_time),
        }
    }
}

/// Legacy single-time behavior as seven identical weekday slots. Early
/// morning times count as the next day by default.
fn derive_weekdays(start_time: &str) -> Vec<WeekdayRule> {
    let next_day = default_next_day(start_time);
    (0..7)
        .map(|_| WeekdayRule {
            time: start_time.into(),
            next_day,
        })
        .collect()
}

fn default_next_day(time: &str) -> bool {
    match (
        NaiveTime::parse_from_str(time, "%H:%M"),
        NaiveTime::parse_from_str(MORNING_BOUNDARY, "%H:%M"),
    ) {
        (Ok(t), Ok(boundary)) => t < boundary,
        _ => true,
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
    let mut config: Config =
        serde_yaml::from_str(yaml).map_err(|e| format!("invalid config: {e}"))?;
    if config.weekdays.is_empty() {
        config.weekdays = derive_weekdays(&config.start_time);
    }
    if config.weekdays.len() != 7 {
        return Err("invalid config: weekdays must have 7 entries".into());
    }
    Ok(config)
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
        let yaml =
            "startTime: \"02:30\"\nsnoozeInterval: 5\nnotification:\n  before: 3\n  duration: 7\n";
        let config = parse(yaml).unwrap();
        assert_eq!(
            config,
            Config {
                start_time: "02:30".into(),
                snooze_interval: 5,
                notification: Notification {
                    before: 3,
                    duration: 7
                },
                weekdays: derive_weekdays("02:30"),
            }
        );
        // 02:30 is before the morning boundary, so it defaults to next day.
        assert!(config.weekdays[0].next_day);
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

    #[test]
    fn derives_weekdays_from_legacy_start_time() {
        let config = parse("startTime: \"23:30\"\n").unwrap();
        assert_eq!(config.weekdays.len(), 7);
        assert!(!config.weekdays[0].next_day, "23:30 stays on the same day");
        let early = parse("startTime: \"01:00\"\n").unwrap();
        assert!(early.weekdays[0].next_day, "01:00 crosses to the next day");
    }

    #[test]
    fn parses_per_weekday_rules() {
        let yaml = "weekdays:\n  - { time: \"01:00\", nextDay: true }\n  - { time: \"23:00\", nextDay: false }\n  - { time: \"01:00\", nextDay: true }\n  - { time: \"01:00\", nextDay: true }\n  - { time: \"01:00\", nextDay: true }\n  - { time: \"02:00\", nextDay: true }\n  - { time: \"00:30\", nextDay: false }\n";
        let config = parse(yaml).unwrap();
        assert_eq!(config.weekdays[1].time, "23:00");
        assert!(!config.weekdays[1].next_day);
        assert_eq!(config.weekdays[6].time, "00:30");
        assert!(
            !config.weekdays[6].next_day,
            "explicit same-day override wins"
        );
    }

    #[test]
    fn rejects_wrong_weekday_count() {
        let yaml = "weekdays:\n  - { time: \"01:00\", nextDay: true }\n";
        assert!(parse(yaml).is_err());
    }
}
