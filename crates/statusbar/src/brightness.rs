//! Brightness pill and its popup slider, driven by the ACPI backlight class.

use std::path::PathBuf;

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};

use crate::popup::Popup;
use crate::sysfs;

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
        let max = sysfs::read_u64(&dir.join(name), "max_brightness").filter(|max| *max > 0);
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
    let max = sysfs::read_u64(&dev, "max_brightness")?;
    let cur = sysfs::read_u64(&dev, "brightness")?;
    if max == 0 {
        return None;
    }
    Some((cur * 100 / max).min(100) as u8)
}

/// Whether a writable backlight device exists. When it does not, the pill is
/// hidden rather than showing a read-only control.
pub fn available() -> bool {
    device().is_some_and(|dev| sysfs::writable(&dev, "brightness"))
}

/// Move the backlight by one fixed percentage step.
pub fn step(up: bool) {
    let Some(dev) = device() else {
        return;
    };
    let Some(max) = sysfs::read_u64(&dev, "max_brightness").filter(|max| *max > 0) else {
        return;
    };
    let cur = sysfs::read_u64(&dev, "brightness").unwrap_or(0);
    let increment = max / 100 * STEP_PERCENT as u64;
    let target = if up {
        cur.saturating_add(increment)
    } else {
        cur.saturating_sub(increment)
    };
    let _ = sysfs::write_u64(&dev, "brightness", target.min(max));
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

        let controls = Box::new(Orientation::Horizontal, 6);
        controls.append(&step_button(&value, false));
        controls.append(&step_button(&value, true));

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
        pop.append(&controls);
    });
}

/// A `+5%`/`-5%` button that nudges the backlight and re-reads the popup's own
/// number. The popup shows the percent alone, where the pill's label is the
/// longer sentence, so the two cannot share one refresh.
fn step_button(value: &Label, up: bool) -> Button {
    let button = Button::with_label(if up { "+5%" } else { "-5%" });
    button.add_css_class("step");
    let value = value.clone();
    button.connect_clicked(move |_| {
        step(up);
        value.set_text(&percent().unwrap_or(0).to_string());
    });
    button
}
