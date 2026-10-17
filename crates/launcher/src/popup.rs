//! The launcher's surface: the query field, the result rows underneath it, and
//! the keys that move through them.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    gdk, glib, ApplicationWindow, Box, Entry, Label, ListBox, ListBoxRow, Orientation,
    PropagationPhase, SelectionMode,
};

use crate::apps::{self, App};
use crate::exec;

#[cfg(test)]
mod tests;

/// The query field, and the list of results that goes under it.
pub(super) fn build() -> (Entry, ListBox) {
    let entry = Entry::new();
    let placeholder = &shell_common::config::get().launcher.placeholder;
    entry.set_placeholder_text(Some(placeholder));
    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Single);
    list.set_activate_on_single_click(true);
    list.set_focusable(false);
    (entry, list)
}

/// Install the launcher's behaviour onto the window: typing refills the list,
/// and the arrow keys, `Return` and `Esc` act on it. The controller sits on the
/// window and captures, so the keys reach it before the field consumes them.
pub(super) fn install(
    window: &ApplicationWindow,
    entry: &Entry,
    list: &ListBox,
    apps: &Rc<Vec<App>>,
) {
    let matches: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));

    let found = matches.clone();
    let found_list = list.clone();
    let found_apps = apps.clone();
    entry.connect_changed(move |entry| {
        *found.borrow_mut() = fill(&found_list, &found_apps, &entry.text());
    });

    let controller = gtk4::EventControllerKey::new();
    controller.set_propagation_phase(PropagationPhase::Capture);
    let key_list = list.clone();
    let key_apps = apps.clone();
    controller.connect_key_pressed(move |_, key, _, _| match key {
        gdk::Key::Escape => std::process::exit(0),
        gdk::Key::Down => {
            move_selection(&key_list, matches.borrow().len(), 1);
            glib::Propagation::Stop
        }
        gdk::Key::Up => {
            move_selection(&key_list, matches.borrow().len(), -1);
            glib::Propagation::Stop
        }
        gdk::Key::Return | gdk::Key::KP_Enter => {
            launch_selected(&key_apps, &matches, &key_list);
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    window.add_controller(controller);
}

/// Refill the list with the apps matching `query`, select the first, and report
/// which apps are on screen now.
pub(super) fn fill(list: &ListBox, apps: &[App], query: &str) -> Vec<usize> {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let found = apps::matches(apps, query);
    for &index in &found {
        list.append(&row(&apps[index]));
    }
    if let Some(first) = list.row_at_index(0) {
        list.select_row(Some(&first));
    }
    found
}

/// One result row: the app's name, and its generic name when it has one.
fn row(app: &App) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_focusable(false);

    let line = Box::new(Orientation::Horizontal, 8);
    let name = Label::new(Some(&app.name));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    line.append(&name);

    if let Some(generic) = &app.generic {
        let suffix = Label::new(Some(generic));
        suffix.add_css_class("dim");
        line.append(&suffix);
    }
    row.set_child(Some(&line));
    row
}

/// Move the selection by `delta` rows, clamped to the list.
fn move_selection(list: &ListBox, count: usize, delta: i32) {
    if count == 0 {
        return;
    }
    let current = list.selected_row().map(|row| row.index()).unwrap_or(0);
    let target = (current + delta).clamp(0, count as i32 - 1);
    if let Some(row) = list.row_at_index(target) {
        list.select_row(Some(&row));
    }
}

/// Launch the selected row's app, then leave: the launcher has done its job.
fn launch_selected(apps: &[App], matches: &RefCell<Vec<usize>>, list: &ListBox) {
    let position = list.selected_row().map(|row| row.index()).unwrap_or(0);
    let found = matches.borrow();
    let Some(&index) = found.get(position as usize) else {
        return;
    };
    if let Err(error) = exec::launch(&apps[index].exec) {
        eprintln!("launcher: {}: {}", apps[index].name, error);
        std::process::exit(1);
    }
    std::process::exit(0);
}
