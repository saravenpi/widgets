//! Unified configuration loader for all desktop widgets.

pub mod types;

use std::path::PathBuf;
use std::sync::OnceLock;

pub use types::*;

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
