//! Where the network state comes from: NetworkManager's `nmcli` when it answers,
//! `networkctl` (systemd-networkd) otherwise, and the kernel's own interfaces
//! when neither daemon is installed — see [`super::link`]. Nothing here formats
//! or draws.

use std::process::Command;

use super::link;

/// A network manager's view of the connection.
pub(super) struct NetState {
    pub(super) kind: StateKind,
    pub(super) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum StateKind {
    Connected,
    Connecting,
    Disconnecting,
    Disconnected,
    Unknown,
}

/// The current state, or `None` when no manager and no interface can answer.
pub(super) fn state() -> Option<NetState> {
    nm_state().or_else(netctl_state).or_else(link::state)
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
        let value = line
            .split_once(':')
            .or_else(|| line.split_once(','))
            .map(|(_, value)| value.trim())
            .unwrap_or("");
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
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let machine = value.get("Machine")?.as_array()?;
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
