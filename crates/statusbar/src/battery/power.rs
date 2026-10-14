//! The battery reading behind the pill: which power-supply device to trust, and
//! what it currently reports.
//!
//! A reading is a snapshot, not a cache. [`Battery::read`] takes the whole thing
//! in one pass — a single directory scan plus the three attributes that matter —
//! so a pill refresh or a popup never probes the same value twice, and the next
//! call still reads the kernel rather than answering from something remembered.

use std::path::PathBuf;

use crate::sysfs;

/// Whether a battery device is present at all.
pub(super) fn present() -> bool {
    device().is_some()
}

/// The first battery device under `/sys/class/power_supply`, if any.
fn device() -> Option<PathBuf> {
    let dir = PathBuf::from("/sys/class/power_supply");
    let rd = std::fs::read_dir(&dir).ok()?;
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
        let capability = sysfs::read_u64(&dir.join(name), "capacity").filter(|cap| *cap <= 100);
        if let Some(cap) = capability {
            if cap > best_cap {
                best_cap = cap;
                best = Some(dir.join(name));
            }
        }
    }
    best
}

/// One battery reading.
pub(super) struct Battery {
    pub(super) pct: u8,
    device: PathBuf,
    status: String,
    on_ac: bool,
}

impl Battery {
    pub(super) fn read() -> Option<Self> {
        let device = device()?;
        Some(Self {
            pct: sysfs::read_u8(&device, "capacity")
                .map(|cap| cap.min(100))
                .unwrap_or(0),
            status: sysfs::read_str(&device, "status").unwrap_or_default(),
            on_ac: sysfs::read_u8(&device, "AC/online").unwrap_or(0) > 0,
            device,
        })
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
#[derive(Clone, Copy)]
pub(super) enum State {
    Charging,
    Discharging,
    Full,
    Plugged,
    Unknown,
}

fn state_of(status: &str, on_ac: bool) -> State {
    let charging = status == "Charging";
    let full = status == "Full";
    let not_charging = status == "Not charging";
    match (charging, full, not_charging, on_ac) {
        (_, true, _, _) => State::Full,
        (true, false, _, true) => State::Charging,
        (false, false, true, true) => State::Plugged,
        (false, false, false, false) => State::Discharging,
        _ => State::Unknown,
    }
}

/// One-word state label for the pill.
pub(super) fn state_label(state: State) -> &'static str {
    match state {
        State::Charging => "Chg",
        State::Discharging => "Dis",
        State::Full => "Full",
        State::Plugged => "AC",
        State::Unknown => "?",
    }
}
