//! Type definitions and defaults for the desktop widgets configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub opacity: f64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            font: "undefined 11".to_string(),
            gap: 10,
            inner_gap: 10,
            opacity: 1.0,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    pub format: String,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: "%H:%M".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StatsConfig {
    pub interval_ms: u64,
    pub cpu: bool,
    pub ram: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModulesConfig {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarConfig {
    pub position: String,
    pub width: i32,
    pub toast_timeout_ms: u64,
    pub max_notifications: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {
    pub width: i32,
    pub top_ratio: f64,
    pub placeholder: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExitConfig {
    pub title: String,
    pub subtitle: String,
    pub commands: ExitCommands,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExitCommands {
    pub exit: String,
    pub reboot: String,
    pub poweroff: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsConfig {
    pub default_tab: String,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            default_tab: "displays".to_string(),
        }
    }
}
