//! Battery pill for the statusbar: the level and state on the bar, and a detail
//! popup on click. The ACPI reading itself lives in [`power`].

mod power;

use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation};

use crate::popup::Popup;
use power::{Battery, State, state_label};

/// Whether a battery device is present at all. When it is not, the pill is hidden
/// rather than showing a dead control.
pub fn available() -> bool {
    power::present()
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
