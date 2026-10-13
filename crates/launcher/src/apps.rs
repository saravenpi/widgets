use std::collections::HashSet;
use std::path::{Path, PathBuf};

const MAX_MATCHES: usize = 10;

pub struct App {
    pub name: String,
    pub generic: Option<String>,
    pub exec: String,
}

pub fn discover() -> Vec<App> {
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for dir in data_dirs() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            let file = entry.file_name().to_string_lossy().into_owned();
            if !seen.insert(file) {
                continue;
            }
            if let Some(app) = parse(&path) {
                found.push(app);
            }
        }
    }
    found.sort_by_key(|app| app.name.to_lowercase());
    found
}

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

fn parse(path: &Path) -> Option<App> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut generic = None;
    let mut exec = None;
    let mut hidden = false;
    let mut in_entry_group = false;
    for line in text.lines() {
        if line.starts_with('[') {
            in_entry_group = line.trim() == "[Desktop Entry]";
            continue;
        }
        if line.starts_with('#') || !in_entry_group {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key {
            "Name" => name = Some(unescape(value)),
            "GenericName" => generic = Some(unescape(value)),
            "Exec" => exec = Some(value.to_string()),
            "NoDisplay" | "Hidden" => hidden = hidden || value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    if hidden {
        return None;
    }
    Some(App {
        name: name?,
        generic,
        exec: exec?,
    })
}

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
mod tests {
    use super::*;

    fn write(name: &str, body: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, body).expect("test desktop file");
        path
    }

    #[test]
    fn parses_fields_and_skips_hidden_entries() {
        let shown = write(
            "launcher-shown.desktop",
            "[Desktop Entry]\nType=Application\nName=Fire\nGenericName=Browser\nExec=firefox %u\n\n[Desktop Action new]\nExec=firefox --new-window\n",
        );
        let app = parse(&shown).expect("visible entry");
        assert_eq!(app.name, "Fire");
        assert_eq!(app.generic.as_deref(), Some("Browser"));
        assert_eq!(app.exec, "firefox %u");
        std::fs::remove_file(&shown).ok();

        let hidden = write(
            "launcher-hidden.desktop",
            "[Desktop Entry]\nName=Secret\nNoDisplay=true\nExec=secret\n",
        );
        assert!(parse(&hidden).is_none());
        std::fs::remove_file(&hidden).ok();

        let localized = write(
            "launcher-localized.desktop",
            "[Desktop Entry]\nName=Term\nName[fr]=Terminale\nExec=kitty\n",
        );
        let app = parse(&localized).expect("localized entry");
        assert_eq!(app.name, "Term");
        std::fs::remove_file(&localized).ok();
    }

    #[test]
    fn matches_fuzzy_substrings_and_caps_results() {
        let apps = vec![
            App {
                name: "Firefox".into(),
                generic: Some("Web Browser".into()),
                exec: "firefox".into(),
            },
            App {
                name: "Files".into(),
                generic: None,
                exec: "nautilus".into(),
            },
        ];
        assert_eq!(matches(&apps, "FIRE"), vec![0]);
        assert_eq!(matches(&apps, "browser"), vec![0]);
        assert_eq!(matches(&apps, ""), vec![0, 1]);
        assert_eq!(matches(&apps, "zzz"), Vec::<usize>::new());
    }
}
