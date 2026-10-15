//! The kernel's own view of the network interfaces, for a machine with no
//! network manager to ask.
//!
//! `nmcli` answers on a machine that runs NetworkManager and `networkctl` on one
//! that runs systemd-networkd. A minimal install — Alpine's default, which is one
//! interface and no daemon at all, or a virtual machine with a single virtio
//! link — has neither, so the probe answered nothing and the pill was not placed:
//! the bar showed a gap where the network state belongs, which reads exactly like
//! a pill whose icon failed to paint.
//!
//! `/sys/class/net` is always there. It carries one directory per interface, and
//! each one has the `operstate` the kernel already knows. The directory is a
//! parameter here rather than a constant read in place, so which interface the
//! bar speaks for is checkable against a directory of the test's own.

use std::path::Path;

use super::status::{NetState, StateKind};

/// Where the kernel publishes its interfaces on a running machine.
const INTERFACES_DIR: &str = "/sys/class/net";

/// The state of the interfaces the kernel lists.
pub(super) fn state() -> Option<NetState> {
    state_at(Path::new(INTERFACES_DIR))
}

/// The same reading, from the interfaces under `root`.
///
/// A machine can carry several — a wired link, a wireless one, the bridges and
/// pairs a container runtime leaves behind, a tunnel from a VPN client — and the
/// one the bar speaks for is the one [`rank`] scores highest. With nothing up,
/// one that is down still answers, so the pill reports the machine as
/// disconnected rather than vanishing.
fn state_at(root: &Path) -> Option<NetState> {
    let mut best: Option<(u8, NetState)> = None;
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(state) = link_state(&entry.path(), &name) else {
            continue;
        };
        let rank = rank(&entry.path(), &state);
        if best.as_ref().is_none_or(|(best, _)| rank > *best) {
            best = Some((rank, state));
        }
    }
    best.map(|(_, state)| state)
}

/// How good a link is to speak for the machine: one that is up beats one that is
/// down, and one the kernel backs with a device beats the bridges, pairs and
/// tunnels that are up on a machine whose own link is not.
fn rank(dir: &Path, state: &NetState) -> u8 {
    let up = u8::from(state.kind == StateKind::Connected);
    let physical = u8::from(dir.join("device").exists());
    up * 2 + physical
}

/// One interface's state, or `None` for loopback and for anything with no
/// `operstate` to read.
fn link_state(dir: &Path, name: &str) -> Option<NetState> {
    if name == "lo" {
        return None;
    }
    let text = std::fs::read_to_string(dir.join("operstate")).ok()?;
    let kind = match text.trim() {
        "up" => StateKind::Connected,
        "dormant" => StateKind::Connecting,
        "down" | "lowerlayerdown" => StateKind::Disconnected,
        _ => StateKind::Unknown,
    };
    Some(NetState {
        kind,
        name: name.to_string(),
    })
}

#[cfg(test)]
mod tests;
