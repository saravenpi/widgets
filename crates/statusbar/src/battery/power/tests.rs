//! The battery reading's checks: which device is chosen, and whether the plugged
//! state survives the trip from sysfs to the pill.
//!
//! They build a `/sys/class/power_supply` of their own under a temp directory and
//! read that, because the real one is whatever machine runs the suite: a desktop
//! has neither a battery nor an adapter, so a check that needed one would fail
//! there. Every fixture therefore has to say out loud what the kernel would have
//! put on the disk.

use std::path::{Path, PathBuf};

use super::*;

/// An online adapter beside a battery the kernel reports as `Not charging`: the
/// machine is plugged in and the battery is not taking charge, which is the
/// reading at a charge threshold. This is the state the bar never showed — the
/// adapter's `online` was read off the battery, where no such file exists, so
/// `Not charging` with no corroboration fell through to `?`.
#[test]
fn an_online_adapter_marks_the_battery_plugged() {
    let root = fixture("plugged");
    supply(&root, "AC", &[("online", "1"), ("type", "Mains")]);
    supply(
        &root,
        "BAT0",
        &[("capacity", "80"), ("status", "Not charging"), ("type", "Battery")],
    );

    let battery = Battery::read_at(&root).expect("the battery is read");

    assert_eq!(battery.pct, 80, "the level comes from the battery");
    assert_eq!(battery.state(), State::Plugged, "and the adapter says plugged");
}

/// The same battery with the adapter unplugged is running on its own cells.
#[test]
fn an_offline_adapter_leaves_the_battery_discharging() {
    let root = fixture("unplugged");
    supply(&root, "AC", &[("online", "0")]);
    supply(&root, "BAT0", &[("capacity", "80"), ("status", "Discharging")]);

    let battery = Battery::read_at(&root).expect("the battery is read");

    assert_eq!(battery.state(), State::Discharging);
}

/// A battery that reports itself charging is charging, with or without an
/// adapter entry to agree with it. Machines exist that expose no adapter at all,
/// and requiring one turned `Charging` into `?` on them.
#[test]
fn charging_does_not_need_an_adapter_entry() {
    let root = fixture("no-adapter");
    supply(&root, "BAT0", &[("capacity", "42"), ("status", "Charging")]);

    let battery = Battery::read_at(&root).expect("the battery is read");

    assert_eq!(battery.state(), State::Charging);
}

/// The adapter is not always called `AC`, so the reading cannot go looking for
/// that one name: `ADP1` here, a USB-C source beside it, and a battery node that
/// carries no `online` of its own in among them.
#[test]
fn the_adapter_is_found_whatever_it_is_named() {
    let root = fixture("named");
    supply(&root, "BAT0", &[("capacity", "55"), ("status", "Not charging")]);
    supply(&root, "BAT1", &[("capacity", "10"), ("status", "Not charging")]);
    supply(&root, "ADP1", &[("online", "1"), ("type", "Mains")]);

    let battery = Battery::read_at(&root).expect("the battery is read");

    assert_eq!(battery.pct, 55, "the fullest battery is the one the bar speaks for");
    assert_eq!(battery.state(), State::Plugged);
}

/// A machine with no battery has nothing to report, so the pill stays off the
/// bar and a popup would say so rather than reading a level of zero.
#[test]
fn a_machine_without_a_battery_reads_as_none() {
    let root = fixture("desktop");
    supply(&root, "AC", &[("online", "1")]);

    assert!(Battery::read_at(&root).is_none());
}

/// A supply directory of our own under a temp directory, named for the check so
/// two of them cannot answer for each other.
fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("widgets-power-{tag}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the fixture directory");
    root
}

/// Write one power supply into `root` with the attributes given. Each attribute
/// is written the way the kernel writes it: the value and a newline, so the
/// reading is exercised against the files it will meet.
fn supply(root: &Path, name: &str, attributes: &[(&str, &str)]) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).expect("the supply directory");
    for (attribute, value) in attributes {
        std::fs::write(dir.join(attribute), format!("{value}\n")).expect("the supply attribute");
    }
    dir
}
