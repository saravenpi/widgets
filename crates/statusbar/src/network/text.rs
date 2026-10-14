//! How the network state reads: the pill's one-liner and the popup's detail.

use super::status::{NetState, StateKind};

/// The pill's label: the state, and the SSID or interface when there is one.
pub(super) fn label(kind: &StateKind, name: &str) -> String {
    let code = match kind {
        StateKind::Connected => "On",
        StateKind::Connecting => "Up",
        StateKind::Disconnecting => "Dn",
        StateKind::Disconnected => "Off",
        StateKind::Unknown => "?",
    };
    if name.is_empty() {
        format!("Net {code}")
    } else {
        format!("Net {code} {}", short_name(name))
    }
}

/// The popup's second line: the state in full, plus the interface.
pub(super) fn detail(state: &NetState) -> String {
    let code = match state.kind {
        StateKind::Connected => "connected",
        StateKind::Connecting => "connecting",
        StateKind::Disconnecting => "disconnecting",
        StateKind::Disconnected => "disconnected",
        StateKind::Unknown => "unknown",
    };
    if state.name.is_empty() {
        format!("state: {code}")
    } else {
        format!("state: {code}\niface: {}", short_name(&state.name))
    }
}

fn short_name(name: &str) -> String {
    let name = name.trim();
    if name.len() <= 16 {
        return name.to_string();
    }
    let mut out = String::with_capacity(18);
    out.push_str(&name[..13]);
    out.push('…');
    out
}
