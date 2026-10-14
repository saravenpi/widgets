//! The sysfs attribute I/O the ACPI-backed pills share.
//!
//! Battery and brightness each read a class directory under `/sys/class` and
//! poke at its attributes. Both used to carry private copies of these four
//! helpers; they live here once so a missing attribute reads as `None` rather
//! than as an error each call site has to special-case.

use std::path::Path;

/// The trimmed contents of `dir/name`, parsed as a `u64`.
pub(crate) fn read_u64(dir: &Path, name: &str) -> Option<u64> {
    std::fs::read_to_string(dir.join(name)).ok()?.trim().parse().ok()
}

/// The trimmed contents of `dir/name`, parsed as a `u8`.
pub(crate) fn read_u8(dir: &Path, name: &str) -> Option<u8> {
    read_u64(dir, name).map(|value| value as u8)
}

/// The trimmed contents of `dir/name`, as text.
pub(crate) fn read_str(dir: &Path, name: &str) -> Option<String> {
    let raw = std::fs::read_to_string(dir.join(name)).ok()?;
    Some(raw.trim().to_string())
}

/// Write `value` to `dir/name`.
pub(crate) fn write_u64(dir: &Path, name: &str, value: u64) -> std::io::Result<()> {
    std::fs::write(dir.join(name), value.to_string())
}

/// Whether `dir/name` can be opened for writing.
///
/// Opening it settles writability without touching the value. A probe that
/// writes to the attribute proves it can write by changing what it reads, which
/// for a backlight means dimming the screen to answer a question about the
/// screen.
pub(crate) fn writable(dir: &Path, name: &str) -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open(dir.join(name))
        .is_ok()
}
