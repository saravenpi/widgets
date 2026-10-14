//! The `.desktop` reading checks: the fields the launcher keeps, the entries it
//! drops, and the query matching.

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
        "[Desktop Entry]\nType=Application\nName=Fire\nGenericName=Browser\n\
         Exec=firefox %u\n\n[Desktop Action new]\nExec=firefox --new-window\n",
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
