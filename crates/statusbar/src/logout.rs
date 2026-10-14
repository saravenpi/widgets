//! Logout pill for the statusbar: ends the current seat/session via `loginctl`.

use std::process::{Command, Stdio};

use gtk4::prelude::*;
use gtk4::Button;

/// End the current session. This is the same action a compositor's
/// `mod+shift+e` logout binding should map to: it asks systemd-logind to
/// terminate the session the bar is running in, which winds down the
/// compositor and its clients cleanly.
///
/// When `loginctl` is absent or the session id is unknown the pill is simply
/// hidden at build time rather than placed on the bar as a dead control.
pub fn available() -> bool {
    let sid = session_id();
    sid.is_some()
        && Command::new("loginctl")
            .arg("terminate-session")
            .arg(sid.unwrap_or_default())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
}

/// The current login session id from the environment, if set. The bar sets this
/// from `$XDG_SESSION_ID` at startup; when that is absent the pill is not
/// placed.
pub fn session_id() -> Option<String> {
    std::env::var("XDG_SESSION_ID").ok().filter(|s| !s.is_empty())
}

/// Build a logout pill. Returns `None` when the session cannot be ended, so the
/// caller can leave the bar without a dead button.
pub fn pill() -> Option<Button> {
    let sid = session_id()?;
    // Verify the tool can actually terminate this session before we advertise the
    // button; a bar that offers logout and then does nothing is worse than none.
    if !available() {
        return None;
    }
    let sid_for_call = sid.clone();
    let button = Button::with_label("Logout");
    button.add_css_class("pill");
    button.add_css_class("logout");
    let sid_clone = sid_for_call.clone();
    button.connect_clicked(move |_| {
        let _ = Command::new("loginctl")
            .arg("terminate-session")
            .arg(&sid_clone)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    });
    Some(button)
}
