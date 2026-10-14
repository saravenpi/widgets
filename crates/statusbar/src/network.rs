//! Network pill for the statusbar: shows the connection state and, when on Wi-Fi,
//! the SSID. Drives NetworkManager's `nmcli` when available and falls back to
//! `networkctl` (systemd-networkd) otherwise. Opens a detail popup on click.

use std::process::Command;

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};

use crate::popup::Popup;

/// Whether a network manager we can talk to is present. When it is not, the pill
/// is hidden rather than showing a dead control.
pub fn available() -> bool {
    state().is_some()
}

/// A short label for the pill: state + optional SSID/iface name.
pub fn label_text() -> Option<String> {
    let s = state()?;
    Some(format_label(&s.kind, &s.name))
}

/// Build the network pill. The pill shows the current state and opens a detail
/// popup on click.
pub fn pill(popup: Popup, label: Label) -> Button {
    let pill = Button::with_label(&label.text());
    pill.add_css_class("pill");
    let popup_for_show = popup.clone();
    let label_for_update = label.clone();
    pill.connect_clicked(move |_| {
        show(&popup_for_show.clone());
        if let Some(text) = label_text() {
            label_for_update.set_text(&text);
        }
    });
    pill
}

/// Refresh a network pill label already on the bar. Used by the ticker.
pub fn refresh(label: &Label) {
    if let Some(text) = label_text() {
        label.set_text(&text);
    }
}

/// Render the network detail popup.
pub fn show(popup: &Popup) {
    let s = match state() {
        Some(s) => s,
        None => {
            popup.show(|pop| {
                let header = Label::new(Some("Network"));
                header.add_css_class("label");
                let value = Label::new(Some("no manager"));
                value.add_css_class("value");
                pop.append(&header);
                pop.append(&value);
            });
            return;
        }
    };
    popup.show(|pop| {
        let header = Label::new(Some("Network"));
        header.add_css_class("label");

        let value = Label::new(Some(&format_label(&s.kind, &s.name)));
        value.add_css_class("value");

        let row = Box::new(Orientation::Vertical, 6);
        let info = Label::new(Some(&detail(&s)));
        info.add_css_class("dim");
        row.append(&info);

        pop.append(&header);
        pop.append(&value);
        pop.append(&row);
    });
}

struct NetState {
    kind: StateKind,
    name: String,
}

#[derive(Clone, Copy, PartialEq)]
enum StateKind {
    Connected,
    Connecting,
    Disconnecting,
    Disconnected,
    Unknown,
}


fn state() -> Option<NetState> {
    nm_state().or_else(|| netctl_state())
}

fn nm_state() -> Option<NetState> {
    let out = Command::new("nmcli")
        .args(["-t", "-f", "STATE", "g"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }
    let kind = match text.as_str() {
        "connected" => StateKind::Connected,
        "connecting" | "connecting (fixed)" => StateKind::Connecting,
        "disconnecting" => StateKind::Disconnecting,
        "disconnected" => StateKind::Disconnected,
        _ => StateKind::Unknown,
    };
    let name = if kind == StateKind::Connected {
        ssid_or_conn_name()?
    } else {
        String::new()
    };
    Some(NetState { kind, name })
}

fn ssid_or_conn_name() -> Option<String> {
    // Prefer the active Wi-Fi SSID.
    ssid_from("dev", "wifi", "active,ssid").or_else(|| {
        // Fallback: active connection name (ethernet, vpn, etc.).
        ssid_from("c", "", "ACTIVE,NAME")
    })
}

fn ssid_from(subcmd: &str, extra: &str, fields: &str) -> Option<String> {
    let args: &[&str] = if extra.is_empty() {
        &["-t", "-f", fields, subcmd]
    } else {
        &["-t", "-f", fields, subcmd, extra]
    };
    let out = Command::new("nmcli").args(args).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("yes") {
            continue;
        }
        let value = line.split_once(':').or_else(|| line.split_once(',')).map(|(_, v)| v.trim()).unwrap_or("");
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn netctl_state() -> Option<NetState> {
    let out = Command::new("networkctl")
        .args(["status", "--json=short"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    parse_networkctl_status(&text)
}

fn parse_networkctl_status(text: &str) -> Option<NetState> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let machine = v.get("Machine")?.as_array()?;
    for iface in machine {
        let ifname = iface.get("Interface")?.as_str()?;
        if ifname == "lo" {
            continue;
        }
        let substate = iface.get("State")?.as_str()?.to_ascii_lowercase();
        let kind = match substate.as_str() {
            "routable" | "configured" => StateKind::Connected,
            "configuring" => StateKind::Connecting,
            "degraded" => StateKind::Connected,
            "off" | "failed" | "absent" => StateKind::Disconnected,
            _ => StateKind::Unknown,
        };
        let name = iface.get("Interface")?.as_str()?.to_string();
        return Some(NetState { kind, name });
    }
    None
}

fn format_label(kind: &StateKind, name: &str) -> String {
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

fn detail(s: &NetState) -> String {
    let code = match s.kind {
        StateKind::Connected => "connected",
        StateKind::Connecting => "connecting",
        StateKind::Disconnecting => "disconnecting",
        StateKind::Disconnected => "disconnected",
        StateKind::Unknown => "unknown",
    };
    if s.name.is_empty() {
        format!("state: {code}")
    } else {
        format!("state: {code}\niface: {}", short_name(&s.name))
    }
}

fn short_name(name: &str) -> String {
    let name = name.trim();
    if name.len() <= 16 {
        return name.to_string();
    }
    let mut out = String::with_capacity(18);
    out.push_str(&name[..13]);
    out.push_str("…");
    out
}
