//! The notification panel: the [`App`] that owns the window's widgets, its
//! construction, and the counters the daemon's events land on.
//!
//! The list itself is drawn in [`list`] and the actions it offers live in
//! [`actions`]; both are `impl App` blocks, so the panel stays one type spread
//! over the files that each have one job.

mod actions;
mod list;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box, Button, EventControllerKey, Label, PolicyType, ScrolledWindow,
};
use gtk4::{gdk, glib};

use crate::dbus::{self, Event};
use crate::{model, popup};

/// The panel and everything on it. The fields are the children of this module
/// too, so `list` and `actions` reach them directly.
pub(super) struct App {
    pub(super) window: ApplicationWindow,
    list: Box,
    scroll: ScrolledWindow,
    count: Label,
    empty: Label,
    clear: Button,
    items: RefCell<Vec<model::Notification>>,
    time_labels: RefCell<Vec<(Label, i64)>>,
    toasts: Rc<popup::Popups>,
    handle: tokio::runtime::Handle,
}

/// Fold one daemon event into the list.
///
/// Both arms end with the same save and redraw, which is what keeps the panel
/// and the history file in step.
pub(super) fn apply_event(app: &Rc<App>, event: &Event) {
    match event {
        Event::Added(notification) => {
            model::insert(&mut app.items.borrow_mut(), notification.clone());
            app.toasts.add(notification.clone());
        }
        Event::Removed(id) => {
            app.items.borrow_mut().retain(|item| item.id != *id);
        }
    }
    model::save(&app.items.borrow());
    App::rebuild(app);
}

impl App {
    pub(super) fn build(
        window: ApplicationWindow,
        items: Vec<model::Notification>,
        handle: tokio::runtime::Handle,
    ) -> Rc<App> {
        let (header, count, clear) = header();
        let (scroll, list) = list();
        let empty = Label::new(Some("No notifications"));
        empty.add_css_class("dim");

        window.set_child(Some(&surface(&header, &scroll, &empty)));

        let application = window
            .application()
            .expect("sidebar: the panel window has an application");
        let toast_handle = handle.clone();
        let toasts = popup::Popups::new(&application, move |id, reason| {
            dbus::emit_closed(&toast_handle, id, reason);
        });

        let app = Rc::new(App {
            window, list, scroll, count, empty, clear, toasts, handle,
            items: RefCell::new(items),
            time_labels: RefCell::new(Vec::new()),
        });
        App::wire(&app);
        App::rebuild(&app);
        app
    }

    /// Hook up what acts on the panel: the clear button, `Esc`, and the timer
    /// that keeps the relative times honest.
    fn wire(app: &Rc<App>) {
        let clear_app = app.clone();
        app.clear.connect_clicked(move |_| App::clear_all(&clear_app));

        let escape_window = app.window.clone();
        let key = EventControllerKey::new();
        key.connect_key_pressed(move |_, pressed, _, _| {
            if pressed == gdk::Key::Escape {
                escape_window.set_visible(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        app.window.add_controller(key);

        let tick_app = app.clone();
        glib::timeout_add_seconds_local(30, move || {
            tick_app.refresh_times();
            glib::ControlFlow::Continue
        });
    }
}

/// The title row: the heading, the count and the clear button.
fn header() -> (Box, Label, Button) {
    let header = Box::new(gtk4::Orientation::Horizontal, 8);
    header.add_css_class("header");

    let title = Label::new(Some("Notifications"));
    title.add_css_class("title");
    title.set_halign(gtk4::Align::Start);
    title.set_hexpand(true);

    let count = Label::new(Some("0"));
    count.add_css_class("count");
    count.add_css_class("dim");

    let clear = Button::with_label("Clear all");
    clear.add_css_class("clear");

    header.append(&title);
    header.append(&count);
    header.append(&clear);
    (header, count, clear)
}

/// The scrolling list every notification row goes into.
fn list() -> (ScrolledWindow, Box) {
    let list = Box::new(gtk4::Orientation::Vertical, 8);
    list.add_css_class("list");

    let scroll = ScrolledWindow::new();
    scroll.set_child(Some(&list));
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_vexpand(true);
    (scroll, list)
}

/// The panel surface: the header, the scrolling list and the empty-state line.
fn surface(header: &Box, scroll: &ScrolledWindow, empty: &Label) -> Box {
    let root = Box::new(gtk4::Orientation::Vertical, 10);
    root.add_css_class("surface");
    root.add_css_class("panel");
    root.append(header);
    root.append(scroll);
    root.append(empty);
    root
}
