//! User settings, persisted as TOML in the platform config directory.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::timer::Durations;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub focus_min: u32,
    pub short_break_min: u32,
    pub long_break_min: u32,
    pub sessions_per_long: u32,
    pub sound: bool,
    /// Chime loudness, 0–100.
    pub volume: u32,
    pub always_on_top: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            focus_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            sessions_per_long: 4,
            sound: true,
            volume: 80,
            always_on_top: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str::<Config>(&s).ok())
            .map(Config::clamped)
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = toml::to_string_pretty(self)
            && let Err(e) = std::fs::write(&path, s)
        {
            eprintln!("could not save settings to {}: {e}", path.display());
        }
    }

    /// Keeps hand-edited values within the ranges the settings view offers.
    pub fn clamped(mut self) -> Self {
        self.focus_min = self.focus_min.clamp(1, 90);
        self.short_break_min = self.short_break_min.clamp(1, 30);
        self.long_break_min = self.long_break_min.clamp(1, 60);
        self.sessions_per_long = self.sessions_per_long.clamp(2, 8);
        self.volume = self.volume.min(100);
        self
    }

    pub fn durations(&self) -> Durations {
        let min = |m: u32| Duration::from_secs(u64::from(m) * 60);
        Durations {
            focus: min(self.focus_min),
            short_break: min(self.short_break_min),
            long_break: min(self.long_break_min),
            sessions_per_long: self.sessions_per_long,
        }
    }
}

fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "Pom").map(|d| d.config_dir().join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_fills_missing_fields() {
        let c = Config { focus_min: 50, ..Config::default() };
        let s = toml::to_string_pretty(&c).unwrap();
        assert_eq!(toml::from_str::<Config>(&s).unwrap(), c);

        let partial: Config = toml::from_str("focus_min = 30").unwrap();
        assert_eq!(partial.focus_min, 30);
        assert_eq!(partial.short_break_min, 5);
    }

    #[test]
    fn clamps_out_of_range_values() {
        let c = Config { focus_min: 0, sessions_per_long: 99, ..Config::default() }.clamped();
        assert_eq!(c.focus_min, 1);
        assert_eq!(c.sessions_per_long, 8);
    }
}
