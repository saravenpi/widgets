//! The panel checks.
//!
//! The panel is real GTK, so it needs a display (Xvfb is enough). The stylesheet
//! check in `shell-common` is display-free on purpose; this one cannot be, so it
//! stands down rather than fail where there is none.

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};

use crate::model;
use crate::popup::tests as toasts;

use super::App;

fn notification(id: u32, urgent: bool) -> model::Notification {
    model::Notification {
        id,
        app_name: "app".into(),
        summary: format!("summary {id}"),
        body: "body".into(),
        ts: model::now(),
        urgent,
    }
}

fn children(parent: &gtk4::Box) -> usize {
    let mut count = 0;
    let mut child = parent.first_child();
    while let Some(current) = child {
        count += 1;
        child = current.next_sibling();
    }
    count
}

/// A panel holding `notifications`, plus the runtime its daemon handle needs.
/// `None` when there is no display to put a panel on.
fn panel(notifications: Vec<model::Notification>) -> Option<(Rc<App>, tokio::runtime::Runtime)> {
    if gtk4::init().is_err() {
        eprintln!("sidebar: no display, skipping the panel test");
        return None;
    }
    let application = Application::builder()
        .application_id("dev.saravenpi.SidebarTest")
        .build();
    application
        .register(None::<&gtk4::gio::Cancellable>)
        .expect("register the test application");
    // `Widget::is_visible` walks the parent chain in GTK4, so the window has to
    // be visible for the panel's own visibility to mean anything.
    let window = ApplicationWindow::new(&application);
    window.set_visible(true);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let app = App::build(window, notifications, runtime.handle().clone());
    Some((app, runtime))
}

/// A loaded panel shows one row per notification and offers to clear them.
fn assert_loaded(panel: &Rc<App>) {
    assert_eq!(panel.count.text().as_str(), "2");
    assert_eq!(children(&panel.list), 2);
    assert!(panel.scroll.is_visible());
    assert!(!panel.empty.is_visible());
    assert!(panel.clear.is_sensitive());
}

/// Dismissing one notification leaves the other one in place.
fn assert_dismissed(panel: &Rc<App>) {
    App::dismiss(panel, 1);
    assert_eq!(panel.count.text().as_str(), "1");
    assert_eq!(children(&panel.list), 1);
}

/// Clearing the panel empties it, and leaves nothing to clear.
fn assert_cleared(panel: &Rc<App>) {
    App::clear_all(panel);
    assert_eq!(panel.count.text().as_str(), "0");
    assert_eq!(children(&panel.list), 0);
    assert!(!panel.scroll.is_visible());
    assert!(panel.empty.is_visible());
    assert!(!panel.clear.is_sensitive());
}

/// A notification that arrives while the panel is closed still has to be
/// visible: the daemon stacks it as a top-right toast and expires it.
fn assert_toasts(panel: &Rc<App>) {
    panel.toasts.add(notification(3, false));
    panel.toasts.add(notification(4, true));
    assert_eq!(toasts::visible(&panel.toasts), 2, "toasts stack in the top-right");
    assert_eq!(toasts::ids(&panel.toasts), vec![4, 3], "the newest is on top");
    assert!(toasts::surface(&panel.toasts).is_visible());

    toasts::force_expiry(&panel.toasts);
    panel.toasts.expire();
    assert_eq!(toasts::visible(&panel.toasts), 0, "an expired toast leaves");
    assert!(!toasts::surface(&panel.toasts).is_visible());
}

/// The panel follows the items it was built with: the count, the rows, the
/// dismissal, the clear, and the toasts.
///
/// The steps are separate functions so each reads on its own, while the state
/// they assert on deliberately flows from one into the next.
#[test]
fn panel_tracks_items() {
    let state = std::env::temp_dir().join("widgets-sidebar-test-state");
    std::env::set_var("XDG_STATE_HOME", &state);

    let Some((panel, _runtime)) = panel(vec![notification(1, false), notification(2, true)]) else {
        return;
    };
    assert_loaded(&panel);
    assert_dismissed(&panel);
    assert_cleared(&panel);
    assert_toasts(&panel);

    let _ = std::fs::remove_dir_all(&state);
}
