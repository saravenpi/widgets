//! Finding the applications installed on this machine.
//!
//! The launcher reads the XDG data directories in precedence order and takes
//! the first `.desktop` file that claims each name, which is what the desktop
//! entry specification asks for.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// How many results the launcher shows at once.
const MAX_MATCHES: usize = 10;

/// One launchable application, as its `.desktop` file describes it.
pub struct App {
    pub name: String,
    pub generic: Option<String>,
    pub exec: String,
}

/// Every application found on this machine, sorted by name.
pub fn discover() -> Vec<App> {
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for dir in data_dirs() {
        for path in unclaimed(&dir, &mut seen) {
            if let Some(app) = parse(&path) {
                found.push(app);
            }
        }
    }
    found.sort_by_key(|app| app.name.to_lowercase());
    found
}

/// The `.desktop` files in one directory, minus the names an earlier directory
/// already contributed: the earlier directory wins.
fn unclaimed(dir: &Path, seen: &mut HashSet<String>) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry.path().extension().and_then(|ext| ext.to_str()) == Some("desktop")
                && seen.insert(entry.file_name().to_string_lossy().into_owned())
        })
        .map(|entry| entry.path())
        .collect()
}

/// The indices of the apps matching `query`, capped at [`MAX_MATCHES`]. An
/// empty query matches everything, so the launcher opens on the top of the list.
pub fn matches(apps: &[App], query: &str) -> Vec<usize> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return (0..apps.len()).take(MAX_MATCHES).collect();
    }
    apps.iter()
        .enumerate()
        .filter(|(_, app)| {
            app.name.to_lowercase().contains(&needle)
                || app
                    .generic
                    .as_deref()
                    .map(|generic| generic.to_lowercase().contains(&needle))
                    .unwrap_or(false)
        })
        .map(|(index, _)| index)
        .take(MAX_MATCHES)
        .collect()
}

/// Where `.desktop` files live, most specific first.
fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        dirs.push(PathBuf::from(xdg).join("applications"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }
    dirs.push(PathBuf::from("/usr/share/applications"));
    dirs
}

/// Read one `.desktop` file. `None` when the entry is hidden, or names no
/// program: a row that cannot be launched is worse than no row.
fn parse(path: &Path) -> Option<App> {
    let text = std::fs::read_to_string(path).ok()?;
    let entry = entry_from(&text);
    if entry.hidden {
        return None;
    }
    Some(App {
        name: entry.name?,
        generic: entry.generic,
        exec: entry.exec?,
    })
}

/// The `[Desktop Entry]` group of a `.desktop` file. Everything after it is a
/// different matter — the actions, and the localized keys like `Name[fr]` — and
/// is skipped.
fn entry_from(text: &str) -> Entry {
    let mut entry = Entry::default();
    let mut in_entry_group = false;
    for line in text.lines() {
        if line.starts_with('[') {
            in_entry_group = line.trim() == "[Desktop Entry]";
            continue;
        }
        if line.starts_with('#') || !in_entry_group {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            entry.apply(key, value.trim());
        }
    }
    entry
}

/// The fields the launcher reads out of a `.desktop` file.
#[derive(Default)]
struct Entry {
    name: Option<String>,
    generic: Option<String>,
    exec: Option<String>,
    hidden: bool,
}

impl Entry {
    /// Fold one `key=value` line in. Keys we do not render are not an error:
    /// the specification defines many more than the launcher shows.
    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "Name" => self.name = Some(unescape(value)),
            "GenericName" => self.generic = Some(unescape(value)),
            "Exec" => self.exec = Some(value.to_string()),
            "NoDisplay" | "Hidden" => self.hidden |= value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
}

/// Resolve the escapes the desktop entry specification defines in string
/// values.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(current) = chars.next() {
        if current != '\\' {
            out.push(current);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests;
