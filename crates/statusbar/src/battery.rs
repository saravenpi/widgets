//! Battery pill for the statusbar: reads the ACPI power supply class and shows a
//! popup with details on click.

use std::path::PathBuf;

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};

use crate::popup::Popup;

/// The first battery device under `/sys/class/power_supply`, if any.
pub fn device() -> Option<PathBuf> {
    let dir = PathBuf::from("/sys/class/power_supply");
    let mut best: Option<PathBuf> = None;
    let mut best_cap: u64 = 0;
    let rd = std::fs::read_dir(&dir).ok()?;
    for entry in rd.flatten() {
        let os_name = entry.file_name();
        let name = match os_name.to_str() {
            Some(n) => n,
            None => continue,
        };
        if !name.starts_with("BAT") {
            continue;
        }
        let cap_path = dir.join(name).join("capacity");
        let cap = match std::fs::read_to_string(&cap_path).ok().and_then(|s| s.trim().parse().ok()) {
            Some(c) if c <= 100 => c,
            _ => continue,
        };
        if cap > best_cap {
            best_cap = cap;
            best = Some(dir.join(name));
        }
    }
    best
}

/// Battery percent (0–100), or `None` when no battery is present.
pub fn percent() -> Option<u8> {
    let dev = device()?;
    let cap = read_u8(&dev, "capacity")?;
    Some(cap.min(100))
}

/// Whether the battery is charging, discharging, full, plugged (on ac but not
/// charging) or unknown.
pub fn state() -> Option<State> {
    let dev = device()?;
    let status = read_str(&dev, "status").unwrap_or_default();
    let on_ac = read_u8(&dev, "AC/online").unwrap_or(0) > 0;
    let charging = matches!(status.as_str(), "Charging");
    let full = matches!(status.as_str(), "Full");
    let not_charging = matches!(status.as_str(), "Not charging");
    Some(match (charging, full, not_charging, on_ac) {
        (_, true, _, _) => State::Full,
        (true, false, _, true) => State::Charging,
        (false, false, true, true) => State::Plugged,
        (false, false, false, false) => State::Discharging,
        _ => State::Unknown,
    })
}

/// Whether a battery device is present at all. When it is not, the pill is hidden
/// rather than showing a dead control.
pub fn available() -> bool {
    device().is_some()
}

/// One-word state label for the pill.
pub fn state_label() -> Option<&'static str> {
    match state() {
        Some(State::Charging) => Some("Chg"),
        Some(State::Discharging) => Some("Dis"),
        Some(State::Full) => Some("Full"),
        Some(State::Plugged) => Some("AC"),
        Some(State::Unknown) => Some("?"),
        None => None,
    }
}

pub enum State {
    Charging,
    Discharging,
    Full,
    Plugged,
    Unknown,
}

fn read_u8(dev: &PathBuf, file: &str) -> Option<u8> {
    read_u64(dev, file).map(|v| v as u8)
}

fn read_u64(dev: &PathBuf, file: &str) -> Option<u64> {
    let path = dev.join(file);
    let s = std::fs::read_to_string(&path).ok()?;
    s.trim().parse().ok()
}

fn read_str(dev: &PathBuf, file: &str) -> Option<String> {
    let path = dev.join(file);
    let s = std::fs::read_to_string(&path).ok()?;
    Some(s.trim().to_string())
}

/// Time remaining (minutes) from `/sys/class/power_supply/BAT0/time_to_full_now`
/// (charging) or `time_to_empty_now` (discharging), or `None`.
pub fn time_remaining() -> Option<u32> {
    let dev = device()?;
    let status = read_str(&dev, "status").unwrap_or_default();
    let raw = if status == "Charging" {
        read_u64(&dev, "time_to_full_now")
    } else {
        read_u64(&dev, "time_to_empty_now")
    };
    raw.map(|m| m.max(0) as u32)
}

/// Build the battery pill. The pill shows the current percent and state, and
/// opens a detail popup on click.
pub fn pill(popup: Popup, label: Label) -> Button {
    let pill = Button::with_label(&label.text());
    pill.add_css_class("pill");
    let popup_for_show = popup.clone();
    let label_for_update = label.clone();
    pill.connect_clicked(move |_| {
        show(&popup_for_show.clone());
        label_for_update.set_text(&format!(
            "Bat {pct}{st}",
            pct = percent().unwrap_or(0),
            st = state_label().unwrap_or(""),
        ));
    });
    pill
}

/// Refresh a battery pill label already on the bar. Used by the ticker.
pub fn refresh(label: &Label) {
    label.set_text(&format!(
        "Bat {pct}{st}",
        pct = percent().unwrap_or(0),
        st = state_label().unwrap_or(""),
    ));
}

/// Render the battery detail popup.
pub fn show(popup: &Popup) {
    let pct = percent().unwrap_or(0);
    let st = state_label().unwrap_or("?");
    let remained = time_remaining().unwrap_or(0);
    popup.show(|pop| {
        let header = Label::new(Some("Battery"));
        header.add_css_class("label");

        let value = Label::new(Some(&format!("{pct}% {st}")));
        value.add_css_class("value");

        let row = Box::new(Orientation::Vertical, 6);

        if remained > 0 {
            let remain_text = match state() {
            Some(State::Charging) => format!("to full in {hh}h {mm}m",
                hh = remained / 60, mm = (remained % 60)),
            Some(State::Discharging) => format!("to empty in {hh}h {mm}m",
                hh = remained / 60, mm = (remained % 60)),
            _ => String::new(),
        };
        let remain = Label::new(Some(&remain_text));
        remain.add_css_class("dim");
        row.append(&remain);
        }

        let capacity_row = Box::new(Orientation::Horizontal, 6);
        let cap_label = Label::new(Some("capacity"));
        cap_label.add_css_class("dim");
        let cap_bar = Box::new(Orientation::Horizontal, 0);
        cap_bar.add_css_class("fill");
        let cap_thumb = Box::new(Orientation::Vertical, 0);
        cap_bar.append(&cap_thumb);
        cap_thumb.set_width_request((pct as f64 / 100.0 * 150.0) as i32);
        capacity_row.append(&cap_label);
        capacity_row.append(&cap_bar);
        row.append(&capacity_row);

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
    });
}
