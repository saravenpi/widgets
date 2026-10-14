//! The launcher surface checks.
//!
//! Rows are real GTK, so these need a display (Xvfb is enough) and stand down
//! where there is none rather than fail on a machine that cannot build a row —
//! the same bargain the sidebar's panel test makes.

use gtk4::prelude::*;

use super::{build, fill, move_selection};
use crate::apps::App;

fn apps() -> Vec<App> {
    vec![
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
    ]
}

fn rows(list: &gtk4::ListBox) -> usize {
    let mut count = 0;
    let mut child = list.first_child();
    while let Some(current) = child {
        count += 1;
        child = current.next_sibling();
    }
    count
}

fn selected(list: &gtk4::ListBox) -> Option<i32> {
    list.selected_row().map(|row| row.index())
}

/// The query decides what is on screen, the first match comes selected, and the
/// arrow keys stay inside the list.
#[test]
fn the_surface_follows_the_query() {
    if gtk4::init().is_err() {
        eprintln!("launcher: no display, skipping the surface test");
        return;
    }
    let (entry, list) = build();
    assert!(entry.text().is_empty(), "the field opens empty");

    let apps = apps();
    assert_eq!(fill(&list, &apps, ""), vec![0, 1]);
    assert_eq!(rows(&list), 2);
    assert_eq!(selected(&list), Some(0), "the first match is selected");

    // A generic name matches too, and the list narrows to it.
    assert_eq!(fill(&list, &apps, "browser"), vec![0]);
    assert_eq!(rows(&list), 1);
    assert_eq!(selected(&list), Some(0));

    move_selection(&list, 1, 1);
    assert_eq!(selected(&list), Some(0), "down at the end stays put");
    move_selection(&list, 1, -1);
    assert_eq!(selected(&list), Some(0), "up at the top stays put");

    assert_eq!(fill(&list, &apps, "zzz"), Vec::<usize>::new());
    assert_eq!(rows(&list), 0);
    assert_eq!(selected(&list), None, "nothing matches, so nothing is selected");
}
