//! Logout pill for the statusbar: ends the current seat/session via `loginctl`.

use std::process::{Command, Stdio};

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};

use crate::icons;

/// Whether the current session can be ended through `loginctl`.
///
/// This is deliberately a *read*: it asks logind to describe the session. An
/// earlier version proved the tool could terminate the session by running
/// `loginctl terminate-session`, which meant the bar logged the user out the
/// moment it was built to decide whether to draw the button.
pub fn available() -> bool {
    let Some(sid) = session_id() else {
        return false;
    };
    Command::new("loginctl")
        .arg("show-session")
        .arg(sid)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// The current login session id from the environment, if set. A session without
/// one cannot be named to `loginctl`, so the pill is not placed at all.
pub fn session_id() -> Option<String> {
    std::env::var("XDG_SESSION_ID").ok().filter(|s| !s.is_empty())
}

/// Build a logout pill. Returns `None` when the session cannot be ended, so the
/// caller can leave the bar without a dead button.
pub fn pill() -> Option<Button> {
    let sid = session_id()?;
    if !available() {
        return None;
    }

    let label = Label::new(Some("Logout"));
    let child = Box::new(Orientation::Horizontal, 4);
    if let Some(image) = icons::image(icons::Icon::Logout) {
        child.append(&image);
    }
    child.append(&label);

    let button = Button::new();
    button.set_child(Some(&child));
    button.add_css_class("pill");
    button.add_css_class("logout");
    button.connect_clicked(move |_| {
        let _ = Command::new("loginctl")
            .arg("terminate-session")
            .arg(&sid)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    });
    Some(button)
}
