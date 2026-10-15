//! What the bar placed and what it did not: `statusbar --diagnose`.
//!
//! A control pill is placed only when its backend answers, and its icon is
//! painted only when the artwork reaches a texture. From outside the bar those
//! two look the same — a gap in the row, or a pill with a label and no glyph —
//! and neither one says anything while it happens. This prints which control is
//! in the row, which is not and why, whether the glyphs painted, the compositor,
//! and whether the machine is virtualised: a backend that is missing and a
//! graphics stack that cannot draw are the two ways a bar ends up without the
//! icons it was built with.
//!
//! It reads and reports; it changes nothing.

use std::fmt::Write as _;

use gtk4::prelude::*;

use crate::{battery, brightness, icons, ipc, logout, network, volume};

/// What to install when `pactl` answers for nothing.
const NO_PACTL: &str =
    "no pactl, or nothing for it to answer for (pulseaudio-utils, pipewire-pulse)";

/// Why a machine may have no backlight, which is every virtual one.
const NO_BACKLIGHT: &str = "nothing writable in /sys/class/backlight (a virtual machine has none)";

/// Why a machine may have no battery, which is every virtual one.
const NO_BATTERY: &str = "no BAT device in /sys/class/power_supply (a virtual machine has none)";

/// Why the kernel's interfaces may not answer either.
const NO_NETWORK: &str = "no NetworkManager, no systemd-networkd, no non-loopback interface";

/// Why the session cannot be ended through `loginctl`.
const NO_LOGINCTL: &str = "$XDG_SESSION_ID is unset, or loginctl is missing (elogind)";

/// The controls the row can carry: each one's name, what a reader should look for
/// when it is in the row, and what to install when it is not.
const CONTROLS: [(&str, &str, &str); 5] = [
    ("volume", "pactl answers for the default sink", NO_PACTL),
    (
        "brightness",
        "a writable backlight is present",
        NO_BACKLIGHT,
    ),
    ("battery", "a BAT device is present", NO_BATTERY),
    ("network", "a link answers", NO_NETWORK),
    ("logout", "loginctl answers for the session", NO_LOGINCTL),
];

/// The vendors whose guests are virtual machines, as the firmware names them.
const VIRTUAL_VENDORS: [&str; 9] = [
    "QEMU",
    "KVM",
    "VMware",
    "VirtualBox",
    "innotek",
    "Apple Virtualization",
    "Parallels",
    "Xen",
    "Microsoft Corporation",
];

/// The whole report, one block per question, in the order someone reads it.
pub fn report() -> String {
    let mut text = String::new();
    let _ = writeln!(
        text,
        "{} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(text, "compositor: {:?}", ipc::detect());
    let _ = writeln!(text, "machine: {}", machine());
    let _ = writeln!(text, "display: {}", display());
    let _ = writeln!(text, "GSK_RENDERER: {}", renderer());
    text.push_str(&pills());
    text.push_str(&glyphs());
    text
}

/// The control pills, each one placed or absent, with the reason either way, and
/// what the network pill is reading — a pill with no label is a second way to
/// look like a control that is not there.
fn pills() -> String {
    let mut text = String::from("pills\n");
    for (name, found, missing) in CONTROLS {
        let _ = writeln!(text, "{}", row(name, placed(name), found, missing));
    }
    let live = network::label_text().unwrap_or_else(|| "nothing to read".to_string());
    let _ = writeln!(text, "  the network pill reads: {live}");
    text
}

/// Whether one control's backend answers, which is what decides its place in the
/// row. The names are the ones [`CONTROLS`] carries.
fn placed(name: &str) -> bool {
    match name {
        "volume" => volume::available(),
        "brightness" => brightness::available(),
        "battery" => battery::available(),
        "network" => network::available(),
        _ => logout::available(),
    }
}

/// One control's line: its name, whether it is in the row, and the reason.
fn row(name: &str, placed: bool, found: &str, missing: &str) -> String {
    let verdict = if placed { "placed" } else { "absent" };
    let why = if placed { found } else { missing };
    format!("  {name:<11}{verdict:<8}{why}")
}

/// Whether every icon reaches a texture, and the size it paints at.
///
/// A texture is built from bytes in memory, so this needs no window — but it does
/// need a display, which is what GTK is initialised for here. Without one the
/// icons cannot be painted at all, and saying that is the honest answer.
fn glyphs() -> String {
    let mut text = String::from("glyphs\n");
    if gtk4::init().is_err() {
        text.push_str("  not checked: no display is up, so nothing can be painted\n");
        return text;
    }
    let tint = icons::tint();
    for icon in icons::ALL {
        let texture = icons::texture(icon, &tint);
        let _ = writeln!(
            text,
            "  {:<11}{}×{}",
            format!("{icon:?}").to_lowercase(),
            texture.width(),
            texture.height()
        );
    }
    text
}

/// What this machine says it is, from the identifiers the firmware publishes.
///
/// A virtual machine is worth naming: on a virtual GPU, GTK4's renderer wants an
/// OpenGL version a guest often does not have, and that shows up as content that
/// never arrives rather than as an error.
fn machine() -> String {
    let id = |field: &str| {
        let path = format!("/sys/class/dmi/id/{field}");
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let name = format!("{} {}", id("sys_vendor"), id("product_name"))
        .trim()
        .to_string();
    if name.is_empty() {
        return "not published".to_string();
    }
    if VIRTUAL_VENDORS.iter().any(|mark| name.contains(mark)) {
        return format!("virtual ({name})");
    }
    name
}

/// Which display the bar is drawing on, since that decides what it can draw with.
fn display() -> String {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return "wayland".to_string();
    }
    if std::env::var_os("DISPLAY").is_some() {
        return "x11, which has no layer surfaces to draw on".to_string();
    }
    "none".to_string()
}

/// The renderer `GSK_RENDERER` names, or the fact that it is unset and GTK picks.
fn renderer() -> String {
    std::env::var("GSK_RENDERER").unwrap_or_else(|_| "unset, GTK chooses".to_string())
}

#[cfg(test)]
mod tests;
