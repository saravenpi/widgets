//! Unified configuration for all desktop widgets.
//!
//! Loads `~/.config/widgets/config.yml` (or `.yaml`), providing typed configuration
//! for `statusbar`, `sidebar`, `launcher`, `exit`, and `settings`.
//! Falls back cleanly to compile-time defaults if the file is absent or incomplete.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub statusbar: StatusbarConfig,
    pub sidebar: SidebarConfig,
    pub launcher: LauncherConfig,
    pub exit: ExitConfig,
    pub settings: SettingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub font: String,
    pub gap: i32,
    pub inner_gap: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusbarConfig {
    pub position: String,
    pub height: i32,
    pub clock: ClockConfig,
    pub stats: StatsConfig,
    pub modules: ModulesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StatsConfig {
    pub interval_ms: u64,
    pub cpu: bool,
    pub ram: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModulesConfig {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarConfig {
    pub position: String,
    pub width: i32,
    pub toast_timeout_ms: u64,
    pub max_notifications: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {
    pub width: i32,
    pub top_ratio: f64,
    pub placeholder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExitConfig {
    pub title: String,
    pub subtitle: String,
    pub commands: ExitCommands,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExitCommands {
    pub exit: String,
    pub reboot: String,
    pub poweroff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsConfig {
    pub default_tab: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            statusbar: StatusbarConfig::default(),
            sidebar: SidebarConfig::default(),
            launcher: LauncherConfig::default(),
            exit: ExitConfig::default(),
            settings: SettingsConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            font: "undefined 11".to_string(),
            gap: 10,
            inner_gap: 10,
        }
    }
}

impl Default for StatusbarConfig {
    fn default() -> Self {
        Self {
            position: "top".to_string(),
            height: 32,
            clock: ClockConfig::default(),
            stats: StatsConfig::default(),
            modules: ModulesConfig::default(),
        }
    }
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: "%H:%M".to_string(),
        }
    }
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            cpu: true,
            ram: true,
        }
    }
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            left: vec!["workspaces".to_string()],
            center: vec!["clock".to_string()],
            right: vec![
                "stats".to_string(),
                "volume".to_string(),
                "brightness".to_string(),
                "battery".to_string(),
                "network".to_string(),
                "logout".to_string(),
            ],
        }
    }
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            position: "right".to_string(),
            width: 380,
            toast_timeout_ms: 5000,
            max_notifications: 50,
        }
    }
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            width: 600,
            top_ratio: 0.35,
            placeholder: "Search applications...".to_string(),
        }
    }
}

impl Default for ExitConfig {
    fn default() -> Self {
        Self {
            title: "Session Control".to_string(),
            subtitle: "End desktop session, reboot, or shut down".to_string(),
            commands: ExitCommands::default(),
        }
    }
}

impl Default for ExitCommands {
    fn default() -> Self {
        Self {
            exit: "swaymsg exit".to_string(),
            reboot: "reboot".to_string(),
            poweroff: "poweroff".to_string(),
        }
    }
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            default_tab: "displays".to_string(),
        }
    }
}

/// Locate the configuration file following XDG standards.
pub fn path() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(&dir).join("widgets/config.yml");
        if p.exists() {
            return Some(p);
        }
        let p2 = PathBuf::from(&dir).join("widgets/config.yaml");
        if p2.exists() {
            return Some(p2);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(&home).join(".config/widgets/config.yml");
        if p.exists() {
            return Some(p);
        }
        let p2 = PathBuf::from(&home).join(".config/widgets/config.yaml");
        if p2.exists() {
            return Some(p2);
        }
    }
    None
}

/// Load the configuration from disk, falling back to defaults.
pub fn load() -> Config {
    if let Some(p) = path() {
        if let Ok(content) = std::fs::read_to_string(&p) {
            match serde_yaml::from_str(&content) {
                Ok(cfg) => return cfg,
                Err(err) => eprintln!("widgets: error parsing config file {}: {err}", p.display()),
            }
        }
    }
    Config::default()
}

/// Access the lazily loaded global configuration instance.
pub fn get() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(load)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_serialize_and_deserialize() {
        let def = Config::default();
        let yaml = serde_yaml::to_string(&def).unwrap();
        assert!(!yaml.is_empty());
        let roundtrip: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(roundtrip.general.gap, 10);
        assert_eq!(roundtrip.statusbar.clock.format, "%H:%M");
        assert_eq!(roundtrip.launcher.width, 600);
    }
}
