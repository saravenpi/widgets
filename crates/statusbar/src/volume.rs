//! Volume pill and its popup slider, driven by PulseAudio / PipeWire via `pactl`.

use std::process::Command;

use gtk4::prelude::*;
use gtk4::{Button, Label, Orientation};

use crate::popup::Popup;

/// Current volume in percent, or `None` when `pactl` is absent or its output is
/// unparseable. We only read `pactl get-sink-volume`; the popup's own buttons
/// call `pactl set-sink-volume`.
pub fn percent() -> Option<u16> {
    let out = Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
        .ok()?;
    parse_percent(&out.stdout)
}

/// Whether `pactl` is present and can talk to a sink. When it is not, the pill
/// is hidden rather than showing a broken control.
pub fn available() -> bool {
    Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
        .is_ok()
}

/// Move the default sink by one fixed step. Each call is its own `pactl` so a
/// held button just repeats the same adjustment.
pub fn step(up: bool) {
    let _ = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", if up { "+5%" } else { "-5%" }]);
}

fn parse_percent(stdout: &[u8]) -> Option<u16> {
    let text = String::from_utf8_lossy(stdout);
    for line in text.lines() {
        let Some(line) = line.strip_prefix("Volume:") else {
            continue;
        };
        let first = line
            .split_whitespace()
            .find_map(parse_channel_percent)?;
        return Some(first.min(100));
    }
    None
}

/// Pick the percent out of a token like `53%/...`.
fn parse_channel_percent(token: &str) -> Option<u16> {
    let Some(percent) = token.strip_suffix("%") else {
        return None;
    };
    percent.trim().parse().ok()
}

/// Set a volume pill label to the current level. Called once at build time and
/// again whenever the pill is clicked.
pub fn refresh(label: &Label) {
    label.set_text(&format!("Vol {}", percent().unwrap_or(0)));
}

/// Render the volume popup contents into a shared `Popup`.
pub fn show(popup: &Popup) {
    let active = percent().unwrap_or(0);
    popup.show(|pop| {
        let header = Label::new(Some("Volume"));
        header.add_css_class("label");

        let value = Label::new(Some(&active.to_string()));
        value.add_css_class("value");

        let row = gtk4::Box::new(Orientation::Horizontal, 6);

        let up = Button::with_label("+1%");
        up.add_css_class("step");
        let value_clone = value.clone();
        up.connect_clicked(move |_| {
            step(true);
            value_clone.set_text(&percent().unwrap_or(0).to_string());
        });

        let fill = crate::popup::fill_bar(active);

        let down = Button::with_label("-1%");
        down.add_css_class("step");
        let value_clone = value.clone();
        down.connect_clicked(move |_| {
            step(false);
            value_clone.set_text(&percent().unwrap_or(0).to_string());
        });

        row.append(&up);
        row.append(&fill);
        row.append(&down);

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
    });
}
