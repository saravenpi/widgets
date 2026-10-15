//! The kernel-interface fallback: which link the pill speaks for, and what it
//! says when the machine has nothing up.

use std::path::Path;

use super::*;

/// One interface to place in a test directory: its name, its `operstate` (`None`
/// for an interface that publishes none) and whether the kernel backs it with a
/// device, which is what makes it hardware rather than a bridge or a tunnel.
struct Interface<'a> {
    name: &'a str,
    operstate: Option<&'a str>,
    physical: bool,
}

/// An interface with nothing but a state, which is virtual and unbacked.
fn plain(name: &'static str, operstate: &'static str) -> Interface<'static> {
    Interface {
        name,
        operstate: Some(operstate),
        physical: false,
    }
}

/// An interface the kernel backs with a device, so it is real hardware.
fn hardware(name: &'static str, operstate: &'static str) -> Interface<'static> {
    Interface {
        name,
        operstate: Some(operstate),
        physical: true,
    }
}

/// Read one test's interfaces and clean them up, whatever the check made of them.
fn report(links: &[Interface], tag: &str) -> Option<NetState> {
    let root = std::env::temp_dir().join(format!("widgets-net-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for link in links {
        let dir = root.join(link.name);
        std::fs::create_dir_all(&dir).expect("create the interface directory");
        if let Some(state) = link.operstate {
            std::fs::write(dir.join("operstate"), format!("{state}\n")).expect("write operstate");
        }
        if link.physical {
            std::fs::create_dir_all(dir.join("device")).expect("create the device link");
        }
    }
    let state = state_at(&root);
    let _ = std::fs::remove_dir_all(&root);
    state
}

/// The kernel's own state is what the pill falls back to, and the link it speaks
/// for is the one carrying traffic: an interface that is up over one that is down,
/// hardware over the bridges and pairs a container runtime leaves behind, and a
/// tunnel when it is the only thing up.
#[test]
fn the_link_that_is_up_is_the_one_reported() {
    let down = [plain("eth0", "down"), hardware("wlan0", "up")];
    let reported = report(&down, "up").expect("an interface to report");
    assert_eq!(
        reported.name, "wlan0",
        "a link that is up outranks one that is down"
    );
    assert_eq!(reported.kind, StateKind::Connected);

    let bridged = [
        plain("docker0", "up"),
        plain("br-07978bb9c153", "up"),
        hardware("enp9s0", "up"),
    ];
    let reported = report(&bridged, "bridge").expect("an interface to report");
    assert_eq!(
        reported.name, "enp9s0",
        "a container bridge is not the machine's link"
    );

    let tunnelled = [hardware("enp9s0", "down"), plain("tailscale0", "up")];
    let reported = report(&tunnelled, "tunnel").expect("an interface to report");
    assert_eq!(
        reported.name, "tailscale0",
        "a tunnel is the link when it is the one up"
    );
}

/// With nothing up the pill still has to say so rather than disappearing: a
/// machine whose link is down is a fact the bar should show, not a gap.
#[test]
fn a_machine_with_nothing_up_reports_a_link_that_is_down() {
    let links = [plain("docker0", "down"), hardware("eth0", "down")];
    let reported = report(&links, "down").expect("an interface to report");
    assert_eq!(
        reported.name, "eth0",
        "hardware answers for a machine with no link"
    );
    assert_eq!(reported.kind, StateKind::Disconnected);
}

/// Loopback is not a connection, an interface with no `operstate` is not one
/// either, and a directory with nothing in it has no link to report.
#[test]
fn a_directory_with_no_link_is_no_state() {
    let stateless = Interface {
        name: "sit0",
        operstate: None,
        physical: false,
    };
    assert!(
        report(&[plain("lo", "unknown"), stateless], "lo-only").is_none(),
        "loopback and stateless interfaces are no link"
    );
    assert!(report(&[], "empty").is_none(), "no interface is no link");
    assert!(
        state_at(Path::new("/nonexistent/widgets-net")).is_none(),
        "a directory that is not there is no link"
    );
}

/// `dormant` is the wireless case: associated but not yet passing traffic, which
/// is neither connected nor down.
#[test]
fn a_dormant_link_is_connecting() {
    let links = [plain("wlan0", "dormant")];
    let reported = report(&links, "dormant").expect("an interface to report");
    assert_eq!(reported.kind, StateKind::Connecting);
}
