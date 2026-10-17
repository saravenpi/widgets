//! Keybindings data model and YAML loader.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindConfig {
    #[serde(default = "default_mod")]
    pub mod_key: String,
    #[serde(default = "default_term")]
    pub terminal: String,
    #[serde(default)]
    pub categories: Vec<Category>,
}

fn default_mod() -> String {
    "Mod4".to_string()
}

fn default_term() -> String {
    "foot".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    #[serde(default)]
    pub binds: Vec<Keybind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybind {
    pub key: String,
    pub desc: String,
    #[serde(default)]
    pub exec: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub sway: Option<String>,
    #[serde(default)]
    pub hypr: Option<String>,
    #[serde(default)]
    pub niri: Option<String>,
}

const DEFAULT_KEYBINDS_YAML: &str = r#"# ~/.config/widgets/keybinds.yml
# Universal keybindings configuration for Sway, Hyprland, and Niri.
# Run `cheatsheet sync` to generate ~/.config/sway/keybinds.conf, etc.

mod_key: "Mod4"
terminal: "foot"

categories:
  - name: "Applications"
    binds:
      - key: "Return"
        desc: "Terminal"
        exec: "$terminal"
      - key: "space"
        desc: "Application Launcher"
        exec: "launcher"
      - key: "n"
        desc: "Notification Sidebar"
        exec: "sidebar toggle"
      - key: "s"
        desc: "Settings & Displays"
        exec: "settings"
      - key: "comma"
        desc: "Settings & Displays"
        exec: "settings"
      - key: "question"
        desc: "Keyboard Cheatsheet"
        exec: "cheatsheet"
      - key: "slash"
        desc: "Keyboard Cheatsheet"
        exec: "cheatsheet"
      - key: "Shift+e"
        desc: "Session Exit Dialog"
        exec: "exit"

  - name: "Window Management"
    binds:
      - key: "q"
        desc: "Close Focused Window"
        action: "close"
      - key: "f"
        desc: "Toggle Fullscreen"
        action: "fullscreen"
      - key: "Shift+r"
        desc: "Reload Compositor Config"
        action: "reload"
      - key: "b"
        desc: "Split Horizontal"
        sway: "splith"
      - key: "v"
        desc: "Split Vertical"
        sway: "splitv"
        hypr: "togglesplit"
      - key: "e"
        desc: "Toggle Split Layout"
        sway: "layout toggle split"
      - key: "a"
        desc: "Focus Parent Container"
        sway: "focus parent"

  - name: "Navigation & Focus"
    binds:
      - key: "h"
        desc: "Focus Left"
        action: "focus_left"
      - key: "j"
        desc: "Focus Down"
        action: "focus_down"
      - key: "k"
        desc: "Focus Up"
        action: "focus_up"
      - key: "l"
        desc: "Focus Right"
        action: "focus_right"
      - key: "Shift+h"
        desc: "Move Window Left"
        action: "move_left"
      - key: "Shift+j"
        desc: "Move Window Down"
        action: "move_down"
      - key: "Shift+k"
        desc: "Move Window Up"
        action: "move_up"
      - key: "Shift+l"
        desc: "Move Window Right"
        action: "move_right"

  - name: "Workspaces"
    binds:
      - key: "1..9"
        desc: "Switch to Workspace 1..9"
        action: "workspace"
      - key: "Shift+1..9"
        desc: "Move Container to Workspace 1..9"
        action: "workspace_move"

  - name: "Hardware & Media"
    binds:
      - key: "XF86AudioRaiseVolume"
        desc: "Volume Up (+5%)"
        exec: "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+"
      - key: "XF86AudioLowerVolume"
        desc: "Volume Down (-5%)"
        exec: "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"
      - key: "XF86AudioMute"
        desc: "Toggle Audio Mute"
        exec: "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"
      - key: "XF86AudioMicMute"
        desc: "Toggle Microphone Mute"
        exec: "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"
      - key: "XF86MonBrightnessUp"
        desc: "Brightness Up (+5%)"
        exec: "brightnessctl set 5%+"
      - key: "XF86MonBrightnessDown"
        desc: "Brightness Down (-5%)"
        exec: "brightnessctl set 5%-"
"#;

pub fn config_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir).join("widgets/keybinds.yml")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/widgets/keybinds.yml")
    } else {
        PathBuf::from("keybinds.yml")
    }
}

pub fn default_config() -> KeybindConfig {
    serde_yaml::from_str(DEFAULT_KEYBINDS_YAML).expect("valid default keybinds YAML")
}

pub fn load() -> KeybindConfig {
    let p = config_path();
    if p.exists() {
        if let Ok(content) = std::fs::read_to_string(&p) {
            match serde_yaml::from_str::<KeybindConfig>(&content) {
                Ok(cfg) => return cfg,
                Err(err) => eprintln!("cheatsheet: error parsing {}: {err}", p.display()),
            }
        }
    }
    // If not present, scaffold the default file so the user can easily discover and edit it
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&p, DEFAULT_KEYBINDS_YAML);
    default_config()
}
