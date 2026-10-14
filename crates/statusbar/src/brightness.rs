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
    let rd = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return None,
    };
    let mut best: Option<PathBuf> = None;
    let mut best_max: u64 = 0;
    for entry in rd.flatten() {
        let os_name = entry.file_name();
    let name = match os_name.to_str() {
        Some(n) => n,
        None => continue,
    };
        // Prefer a real backlight interface (native, intel_backlight, amdgpu_bl,
        // nv_panel, panfrost, etc.) over the generic `virtual` one.
        if name.starts_with("virtual") {
            continue;
        }
        let max_path = dir.join(name).join("max_brightness");
        let max = match std::fs::read_to_string(&max_path).ok().and_then(|s| s.trim().parse().ok()) {
            Some(m) if m > 0 => m,
            _ => continue,
        };
        if max > best_max {
            best_max = max;
            best = Some(dir.join(name));
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
pub fn available() -> bool {
    device().is_some_and(|dev| write_u64(&dev, "brightness", 0).is_ok())
}

/// Move the backlight by one fixed percentage step.
pub fn step(up: bool) {
    let dev = match device() {
        Some(d) => d,
        None => return,
    };
    let max = match read_u64(&dev, "max_brightness") {
        Some(m) if m > 0 => m,
        _ => return,
    };
    let cur = read_u64(&dev, "brightness").unwrap_or(0);
    let target = if up {
        cur.saturating_add(max / 100 * STEP_PERCENT as u64)
    } else {
        cur.saturating_sub(max / 100 * STEP_PERCENT as u64)
    };
    let _ = write_u64(&dev, "brightness", target.min(max));
}

fn read_u64(dev: &PathBuf, file: &str) -> Option<u64> {
    let path = dev.join(file);
    let s = std::fs::read_to_string(&path).ok()?;
    s.trim().parse().ok()
}

fn write_u64(dev: &PathBuf, file: &str, value: u64) -> Result<(), std::io::Error> {
    let path = dev.join(file);
    std::fs::write(&path, value.to_string())
}

/// Build the brightness pill that sits in the bar. The pill shows the current
/// percent and opens the shared popup on click.
pub fn pill(popup: Popup, label: Label) -> Button {
    let pill = Button::with_label(&label.text());
    pill.add_css_class("pill");
    let popup_for_show = popup.clone();
    let label_for_update = label.clone();        pill.connect_clicked(move |_| {
            show(&popup_for_show.clone());
            label_for_update.set_text(&format!("Brightness {}", percent().unwrap_or(0)));
        });
    pill
}

/// Render the brightness popup contents into a shared `Popup`.
pub fn show(popup: &Popup) {
    let active = percent().unwrap_or(0);
    popup.show(|pop| {
        let header = Label::new(Some("Brightness"));
        header.add_css_class("label");

        let value = Label::new(Some(&format!("{active}")));
        value.add_css_class("value");

        let row = Box::new(Orientation::Vertical, 6);

        let bar = Box::new(Orientation::Horizontal, 0);
        bar.add_css_class("fill");
        let thumb = gtk4::Box::new(Orientation::Vertical, 0);
        bar.append(&thumb);
        thumb.set_width_request((active as f64 / 100.0 * 150.0) as i32);
        row.append(&bar);

        let controls = Box::new(Orientation::Horizontal, 6);
        let up = Button::with_label("+5%");
        up.add_css_class("step");
        let value_clone = value.clone();
        up.connect_clicked(move |_| {
            step(true);
            value_clone.set_text(&format!("{}", percent().unwrap_or(0)));
        });
        let down = Button::with_label("-5%");
        down.add_css_class("step");
        let value_clone = value.clone();
        down.connect_clicked(move |_| {
            step(false);
            value_clone.set_text(&format!("{}", percent().unwrap_or(0)));
        });
        controls.append(&down);
        controls.append(&up);

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
        pop.append(&controls);
    });
}
