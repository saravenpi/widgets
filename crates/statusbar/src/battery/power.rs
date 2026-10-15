//! The battery reading behind the pill: which power-supply device to trust, and
//! what it currently reports.
//!
//! A reading is a snapshot, not a cache. [`Battery::read`] takes the whole thing
//! in one pass — a single directory scan plus the attributes that matter — so a
//! pill refresh or a popup never probes the same value twice, and the next call
//! still reads the kernel rather than answering from something remembered.
//!
//! The plugged state comes from two devices. The battery reports the level and
//! whether current is flowing; the adapter — `AC`, `ACAD`, `ADP1`, named per
//! machine, with a USB-C source beside it — reports whether it is online. They
//! are separate entries under `/sys/class/power_supply`, and a reading takes both.
//!
//! The kernel publishes these as whole *devices* rather than as attributes of the
//! battery, so the supply directory is a parameter here rather than a constant
//! read in place: both halves of the reading are then checkable against a
//! directory of the test's own, which is the only way to test them on a machine
//! that has no battery.

use std::path::{Path, PathBuf};

use crate::sysfs;

/// Where the kernel publishes its power supplies on a running machine.
const SUPPLY_DIR: &str = "/sys/class/power_supply";

/// Whether a battery device is present at all.
pub(super) fn present() -> bool {
    device(Path::new(SUPPLY_DIR)).is_some()
}

/// The battery device under `root`, if any.
///
/// A machine can carry more than one (a second battery in a bay, a peripheral
/// that presents itself as one), and they are listed in no particular order, so
/// the fullest one is the one the bar speaks for.
fn device(root: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(root).ok()?;
    let mut best: Option<PathBuf> = None;
    let mut best_cap: u64 = 0;
    for entry in rd.flatten() {
        let os_name = entry.file_name();
        let Some(name) = os_name.to_str() else {
            continue;
        };
        if !name.starts_with("BAT") {
            continue;
        }
        let capability = sysfs::read_u64(&root.join(name), "capacity").filter(|cap| *cap <= 100);
        if let Some(cap) = capability {
            if cap > best_cap {
                best_cap = cap;
                best = Some(root.join(name));
            }
        }
    }
    best
}

/// Whether an external supply is feeding the machine.
///
/// `online` is the *adapter's* attribute, and the adapter is a device of its own
/// under `root`. Asking the battery directory for `AC/online` — a directory that
/// does not exist, on a device that never carries it — never resolved, so the
/// read fell back to "on battery" on every machine: the bar could see a battery
/// charging and still not say the machine was plugged in.
///
/// Taken as a scan rather than a fixed name because the adapter is `AC` on one
/// machine, `ACAD` on another and `ADP1` on a third, and a USB-C source is a
/// fourth entry again. A battery has no `online` file, so no device-type test is
/// needed to tell them apart.
fn online(root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        sysfs::read_u64(&root.join(entry.file_name()), "online").is_some_and(|value| value > 0)
    })
}

/// One battery reading.
pub(super) struct Battery {
    pub(super) pct: u8,
    device: PathBuf,
    status: String,
    on_ac: bool,
}

impl Battery {
    /// The battery under `root`, read in one pass.
    fn read_at(root: &Path) -> Option<Self> {
        let device = device(root)?;
        Some(Self {
            pct: sysfs::read_u8(&device, "capacity")
                .map(|cap| cap.min(100))
                .unwrap_or(0),
            status: sysfs::read_str(&device, "status").unwrap_or_default(),
            on_ac: online(root),
            device,
        })
    }

    /// The same reading, from the supplies this machine actually has.
    pub(super) fn read() -> Option<Self> {
        Self::read_at(Path::new(SUPPLY_DIR))
    }

    pub(super) fn state(&self) -> State {
        state_of(&self.status, self.on_ac)
    }

    /// Time remaining in minutes, from `time_to_full_now` while charging and
    /// `time_to_empty_now` otherwise. `None` when the kernel reports nothing.
    pub(super) fn time_remaining(&self) -> Option<u32> {
        let file = if self.status == "Charging" {
            "time_to_full_now"
        } else {
            "time_to_empty_now"
        };
        sysfs::read_u64(&self.device, file).map(|minutes| minutes as u32)
    }
}

/// Whether the battery is charging, discharging, full, plugged (on ac but not
/// charging) or unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    Charging,
    Discharging,
    Full,
    Plugged,
    Unknown,
}

/// The state a `status` and an adapter reading describe.
///
/// The battery's own `status` is taken at its word: a battery reports `Charging`
/// only while current is flowing into it, so an adapter entry is corroboration
/// rather than evidence, and there are machines with no adapter entry at all —
/// they used to show `?` while charging. `Plugged` is the one state that needs
/// the adapter: it is what `Not charging` means when something is feeding the
/// machine, the usual reading at a charge threshold.
fn state_of(status: &str, on_ac: bool) -> State {
    match status {
        "Full" => State::Full,
        "Charging" => State::Charging,
        "Discharging" => State::Discharging,
        "Not charging" if on_ac => State::Plugged,
        _ => State::Unknown,
    }
}

#[cfg(test)]
mod tests;
