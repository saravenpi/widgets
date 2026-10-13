mod hyprland;
mod niri;
mod sway;

use async_channel::Sender;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compositor {
    Sway,
    Niri,
    Hyprland,
    Unknown,
}

#[derive(Clone)]
pub struct Workspace {
    pub key: String,
    pub label: String,
    pub focused: bool,
    pub urgent: bool,
}

pub fn detect() -> Compositor {
    if env_set("SWAYSOCK") {
        Compositor::Sway
    } else if env_set("NIRI_SOCKET") {
        Compositor::Niri
    } else if env_set("HYPRLAND_INSTANCE_SIGNATURE") {
        Compositor::Hyprland
    } else {
        Compositor::Unknown
    }
}

pub fn name(compositor: Compositor) -> &'static str {
    match compositor {
        Compositor::Sway => "sway",
        Compositor::Niri => "niri",
        Compositor::Hyprland => "hyprland",
        Compositor::Unknown => "wayland",
    }
}

pub fn start(compositor: Compositor, tx: Sender<Vec<Workspace>>) {
    match compositor {
        Compositor::Sway => drop(std::thread::spawn(move || sway::run(tx))),
        Compositor::Niri => drop(std::thread::spawn(move || niri::run(tx))),
        Compositor::Hyprland => drop(std::thread::spawn(move || hyprland::run(tx))),
        Compositor::Unknown => {}
    }
}

pub fn focus(compositor: Compositor, key: &str) {
    match compositor {
        Compositor::Sway => sway::focus(key),
        Compositor::Niri => niri::focus(key),
        Compositor::Hyprland => hyprland::focus(key),
        Compositor::Unknown => {}
    }
}

fn env_set(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|value| !value.is_empty())
}
