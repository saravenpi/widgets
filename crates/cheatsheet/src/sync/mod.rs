//! Synchronization of abstract keybindings into compositor configurations.

mod hypr;
mod niri;
mod sway;

pub use hypr::generate_hypr;
pub use niri::generate_niri;
pub use sway::generate_sway;

use std::path::PathBuf;
use crate::model::KeybindConfig;

pub fn sync_all(config: &KeybindConfig) -> Result<Vec<PathBuf>, String> {
    let mut updated = Vec::new();
    let home = std::env::var("HOME").map(PathBuf::from).map_err(|e| e.to_string())?;

    // 1. Sway
    let sway_path = home.join(".config/sway/keybinds.conf");
    write_config(&sway_path, generate_sway(config))?;
    updated.push(sway_path);

    // 2. Hyprland
    let hypr_path = home.join(".config/hypr/keybinds.conf");
    write_config(&hypr_path, generate_hypr(config))?;
    updated.push(hypr_path);

    // 3. Niri
    let niri_path = home.join(".config/niri/keybinds.kdl");
    write_config(&niri_path, generate_niri(config))?;
    updated.push(niri_path);

    Ok(updated)
}

fn write_config(path: &PathBuf, content: String) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, content).map_err(|e| format!("Writing {}: {e}", path.display()))
}
