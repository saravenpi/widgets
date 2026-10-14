//! Brightness pill and its popup slider, driven by the ACPI backlight class.

use std::path::PathBuf;

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};

use crate::popup::Popup;

const STEP_PERCENT: u8 = 5;

/// The first available backlight device, or `None` when there is no
/// `/sys/class/backlight` on this machine.
pub fn device() -> Option<PathBuf> {
    let dir = PathBuf::from("/sys/class/backlight");
    let rd = std::fs::read_dir(&dir).ok()?;
    let mut best: Option<PathBuf> = None;
    let mut best_max: u64 = 0;
    for entry in rd.flatten() {
        let os_name = entry.file_name();
        let name = os_name.to_str()?;
        // Prefer a real backlight interface (native, intel_backlight, amdgpu_bl,
        // nv_panel, panfrost, etc.) over the generic `virtual` one.
        if name.starts_with("virtual") {
            continue;
        }
        let max = read_u64(&dir.join(name), "max_brightness").filter(|max| *max > 0);
        if let Some(max) = max {
            if max > best_max {
                best_max = max;
                best = Some(dir.join(name));
            }
        }
    }
    best
}

/// Current brightness as a percent of `max_brightness`, or `None` when no
/// backlight device is reachable.
pub fn percent() -> Option<u8> {
    let dev = device()?;
    let max = read_u64(&dev, "max_brightness")?;
    let cur = read_u64(&dev, "brightness")?;
    if max == 0 {
        return None;
    }
    Some((cur * 100 / max).min(100) as u8)
}

/// Whether a writable backlight device exists. When it does not, the pill is
/// hidden rather than showing a read-only control.
///
/// Writability is settled by opening `brightness` for writing, never by writing
/// to it: a probe that dims the screen to prove it can dim the screen is worse
/// than no probe at all.
pub fn available() -> bool {
    device()
        .is_some_and(|dev| std::fs::OpenOptions::new().write(true).open(dev.join("brightness")).is_ok())
}

/// Move the backlight by one fixed percentage step.
pub fn step(up: bool) {
    let Some(dev) = device() else {
        return;
    };
    let Some(max) = read_u64(&dev, "max_brightness").filter(|max| *max > 0) else {
        return;
    };
    let cur = read_u64(&dev, "brightness").unwrap_or(0);
    let step = max / 100 * STEP_PERCENT as u64;
    let target = if up {
        cur.saturating_add(step)
    } else {
        cur.saturating_sub(step)
    };
    let _ = write_u64(&dev, "brightness", target.min(max));
}

fn read_u64(dev: &PathBuf, file: &str) -> Option<u64> {
    std::fs::read_to_string(dev.join(file)).ok()?.trim().parse().ok()
}

fn write_u64(dev: &PathBuf, file: &str, value: u64) -> Result<(), std::io::Error> {
    std::fs::write(dev.join(file), value.to_string())
}

/// Set a brightness pill label to the current level. Called once at build time
/// and again whenever the pill is clicked.
pub fn refresh(label: &Label) {
    label.set_text(&match percent() {
        Some(pct) => format!("Brightness {pct}"),
        None => "Brightness ?".to_string(),
    });
}

/// Render the brightness popup contents into a shared `Popup`.
pub fn show(popup: &Popup) {
    let Some(active) = percent() else {
        crate::popup::message(popup, "Brightness", "unavailable");
        return;
    };

    popup.show(|pop| {
        let header = Label::new(Some("Brightness"));
        header.add_css_class("label");

        let value = Label::new(Some(&active.to_string()));
        value.add_css_class("value");

        let row = Box::new(Orientation::Vertical, 6);

        row.append(&crate::popup::fill_bar(active.into()));

        // The popup's number is the percent alone; the pill's label is the
        // longer sentence, so the two cannot share one refresh.
        let percent_text = || percent().unwrap_or(0).to_string();

        let up = Button::with_label("+5%");
        up.add_css_class("step");
        let value_clone = value.clone();
        up.connect_clicked(move |_| {
            step(true);
            value_clone.set_text(&percent_text());
        });

        let down = Button::with_label("-5%");
        down.add_css_class("step");
        let value_clone = value.clone();
        down.connect_clicked(move |_| {
            step(false);
            value_clone.set_text(&percent_text());
        });

        let controls = Box::new(Orientation::Horizontal, 6);
        controls.append(&down);
        controls.append(&up);

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
        pop.append(&controls);
    });
}
