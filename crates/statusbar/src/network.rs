//! Network pill for the statusbar: shows the connection state and, when on Wi-Fi,
//! the SSID, and opens a detail popup on click. The probing and the wording live
//! in [`status`] and [`text`].

mod status;
mod text;

use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation};

use crate::popup::Popup;

/// Whether a network manager we can talk to is present. When it is not, the pill
/// is hidden rather than showing a dead control.
pub fn available() -> bool {
    status::state().is_some()
}

/// A short label for the pill: state + optional SSID/iface name.
pub fn label_text() -> Option<String> {
    let state = status::state()?;
    Some(text::label(&state.kind, &state.name))
}

/// Set a network pill label to the current state. Called once at build time and
/// again on every tick. Leaves the label alone when the state cannot be read,
/// rather than blanking a value that was correct a moment ago.
pub fn refresh(label: &Label) {
    if let Some(current) = label_text() {
        label.set_text(&current);
    }
}

/// Render the network detail popup.
pub fn show(popup: &Popup) {
    let Some(state) = status::state() else {
        crate::popup::message(popup, "Network", "no manager");
        return;
    };

    let summary = text::label(&state.kind, &state.name);
    let detail = text::detail(&state);

    popup.show(|pop| {
        let header = Label::new(Some("Network"));
        header.add_css_class("label");

        let value = Label::new(Some(&summary));
        value.add_css_class("value");

        let row = Box::new(Orientation::Vertical, 6);
        let info = Label::new(Some(&detail));
        info.add_css_class("dim");
        row.append(&info);

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
    });
}
