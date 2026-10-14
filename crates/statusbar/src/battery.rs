//! Battery pill for the statusbar: reads the ACPI power supply class and shows a
//! popup with details on click.

use std::path::PathBuf;

use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation};

use crate::popup::Popup;

/// The first battery device under `/sys/class/power_supply`, if any.
fn device() -> Option<PathBuf> {
    let dir = PathBuf::from("/sys/class/power_supply");
    let rd = std::fs::read_dir(&dir).ok()?;
    let mut best: Option<PathBuf> = None;
    let mut best_cap: u64 = 0;
    for entry in rd.flatten() {
        let os_name = entry.file_name();
        let Some(name) = os_name.to_str() else {
            continue;
        };
        if !name.starts_with("BAT") {
            continue;
        }
        let capability = read_u64(&dir.join(name), "capacity").filter(|cap| *cap <= 100);
        if let Some(cap) = capability {
            if cap > best_cap {
                best_cap = cap;
                best = Some(dir.join(name));
            }
        }
    }
    best
}

/// One battery reading.
///
/// [`Battery::read`] takes the whole snapshot in a single pass — one directory
/// scan and three attribute reads — so refreshing a pill or opening the popup
/// never probes the same value twice. The value is a snapshot, not a cache: the
/// next call always reads the kernel again.
struct Battery {
    device: PathBuf,
    pct: u8,
    status: String,
    on_ac: bool,
}

impl Battery {
    fn read() -> Option<Self> {
        let device = device()?;
        Some(Self {
            pct: read_u8(&device, "capacity").map(|cap| cap.min(100)).unwrap_or(0),
            status: read_str(&device, "status").unwrap_or_default(),
            on_ac: read_u8(&device, "AC/online").unwrap_or(0) > 0,
            device,
        })
    }

    fn state(&self) -> State {
        state_of(&self.status, self.on_ac)
    }

    /// Time remaining in minutes, from `time_to_full_now` while charging and
    /// `time_to_empty_now` otherwise. `None` when the kernel reports nothing.
    fn time_remaining(&self) -> Option<u32> {
        let file = if self.status == "Charging" {
            "time_to_full_now"
        } else {
            "time_to_empty_now"
        };
        read_u64(&self.device, file).map(|minutes| minutes as u32)
    }
}

/// Whether the battery is charging, discharging, full, plugged (on ac but not
/// charging) or unknown.
#[derive(Clone, Copy)]
enum State {
    Charging,
    Discharging,
    Full,
    Plugged,
    Unknown,
}

fn state_of(status: &str, on_ac: bool) -> State {
    let charging = status == "Charging";
    let full = status == "Full";
    let not_charging = status == "Not charging";
    match (charging, full, not_charging, on_ac) {
        (_, true, _, _) => State::Full,
        (true, false, _, true) => State::Charging,
        (false, false, true, true) => State::Plugged,
        (false, false, false, false) => State::Discharging,
        _ => State::Unknown,
    }
}

/// One-word state label for the pill.
fn state_label(state: State) -> &'static str {
    match state {
        State::Charging => "Chg",
        State::Discharging => "Dis",
        State::Full => "Full",
        State::Plugged => "AC",
        State::Unknown => "?",
    }
}

fn read_str(dev: &PathBuf, file: &str) -> Option<String> {
    let raw = std::fs::read_to_string(dev.join(file)).ok()?;
    Some(raw.trim().to_string())
}

fn read_u8(dev: &PathBuf, file: &str) -> Option<u8> {
    read_u64(dev, file).map(|value| value as u8)
}

fn read_u64(dev: &PathBuf, file: &str) -> Option<u64> {
    std::fs::read_to_string(dev.join(file)).ok()?.trim().parse().ok()
}

/// Whether a battery device is present at all. When it is not, the pill is
/// hidden rather than showing a dead control.
pub fn available() -> bool {
    device().is_some()
}

/// Set a battery pill label to the current level and state. Called once at build
/// time and again on every tick.
pub fn refresh(label: &Label) {
    let text = match Battery::read() {
        Some(battery) => format!(
            "Bat {pct}{st}",
            pct = battery.pct,
            st = state_label(battery.state())
        ),
        None => "Bat ?".to_string(),
    };
    label.set_text(&text);
}

/// The "to full in 1h 20m" line, when the kernel reports a time and the state
/// gives that time a meaning. Nothing to say while full or in an unknown state,
/// so those fall through to `None` and the popup simply omits the line.
fn remaining_text(state: State, minutes: Option<u32>) -> Option<String> {
    let minutes = minutes.filter(|minutes| *minutes > 0)?;
    let prefix = match state {
        State::Charging => "to full in",
        State::Discharging | State::Plugged => "to empty in",
        State::Full | State::Unknown => return None,
    };
    Some(format!(
        "{prefix} {hh}h {mm}m",
        hh = minutes / 60,
        mm = minutes % 60
    ))
}

/// Render the battery detail popup.
pub fn show(popup: &Popup) {
    let Some(battery) = Battery::read() else {
        crate::popup::message(popup, "Battery", "none");
        return;
    };

    let pct = battery.pct;
    let state = battery.state();
    let state_text = state_label(state);
    let remained = battery.time_remaining().filter(|minutes| *minutes > 0);

    popup.show(|pop| {
        let header = Label::new(Some("Battery"));
        header.add_css_class("label");

        let value = Label::new(Some(&format!("{pct}% {state_text}")));
        value.add_css_class("value");

        let row = Box::new(Orientation::Vertical, 6);

        if let Some(text) = remaining_text(state, remained) {
            let remain = Label::new(Some(&text));
            remain.add_css_class("dim");
            row.append(&remain);
        }

        let capacity_row = Box::new(Orientation::Horizontal, 6);
        let capacity_label = Label::new(Some("capacity"));
        capacity_label.add_css_class("dim");
        capacity_row.append(&capacity_label);
        capacity_row.append(&crate::popup::fill_bar(pct.into()));
        row.append(&capacity_row);

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
    });
}
