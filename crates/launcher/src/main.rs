mod apps;
mod exec;

use std::cell::RefCell;
use std::rc::Rc;

use apps::App;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, Entry, Label, ListBox, ListBoxRow, Orientation, PropagationPhase,
    SelectionMode,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use shell_common::app::run as run_app;
use shell_common::layer::{ensure_supported, LayerWindow};
use shell_common::theme;

const STYLE: &str = include_str!("../style.css");
const WIDTH: i32 = 600;
const TOP_RATIO: f64 = 0.35;

fn main() {
    if std::env::args().skip(1).any(|argument| argument == "--version") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }
    let apps = Rc::new(apps::discover());
    let application = Application::builder()
        .application_id("dev.saravenpi.Launcher")
        .build();
    let shared = apps.clone();
    application.connect_activate(move |app| activate(app, &shared));
    let _ = run_app(&application, env!("CARGO_PKG_NAME"));
}

fn activate(application: &Application, apps: &Rc<Vec<App>>) {
    if !ensure_supported() {
        std::process::exit(1);
    }
    theme::init(STYLE);

    let launcher = LayerWindow::new(application).layer(Layer::Overlay);
    launcher.anchors(true, true, true, false);
    launcher.exclusive_zone(false);
    launcher.keyboard_mode(KeyboardMode::Exclusive);
    let window = launcher.window.clone();
    let (top, side) = margins();
    window.set_margin(Edge::Top, top);
    window.set_margin(Edge::Left, side);
    window.set_margin(Edge::Right, side);

    let entry = Entry::new();
    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Single);
    list.set_activate_on_single_click(true);
    list.set_focusable(false);
    let matches = Rc::new(RefCell::new(Vec::new()));

    {
        let list = list.clone();
        let apps = apps.clone();
        let matches = matches.clone();
        entry.connect_changed(move |entry| {
            let found = rebuild(&list, &apps, &entry.text());
            *matches.borrow_mut() = found;
        });
    }

    let controller = gtk4::EventControllerKey::new();
    controller.set_propagation_phase(PropagationPhase::Capture);
    {
        let list = list.clone();
        let apps = apps.clone();
        let matches = matches.clone();
        controller.connect_key_pressed(move |_, key, _, _| match key {
            gdk::Key::Escape => std::process::exit(0),
            gdk::Key::Down => {
                let count = matches.borrow().len();
                move_selection(&list, count, 1);
                glib::Propagation::Stop
            }
            gdk::Key::Up => {
                let count = matches.borrow().len();
                move_selection(&list, count, -1);
                glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                launch_selected(&apps, &matches, &list);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        });
    }
    window.add_controller(controller);

    let popup = gtk4::Box::new(Orientation::Vertical, 0);
    popup.add_css_class("surface");
    popup.add_css_class("popup");
    popup.set_width_request(WIDTH);
    popup.append(&entry);
    popup.append(&list);
    window.set_child(Some(&popup));

    rebuild(&list, apps, "");
    window.present();
    entry.grab_focus();
}

fn rebuild(list: &ListBox, apps: &[App], query: &str) -> Vec<usize> {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let found = apps::matches(apps, query);
    for &index in &found {
        let app = &apps[index];
        let row = ListBoxRow::new();
        row.set_focusable(false);
        let line = gtk4::Box::new(Orientation::Horizontal, 8);
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
        list.append(&row);
    }
    if let Some(row) = list.row_at_index(0) {
        list.select_row(Some(&row));
    }
    found
}

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

fn margins() -> (i32, i32) {
    let display = match gdk::Display::default() {
        Some(display) => display,
        None => return (0, 0),
    };
    let monitors = display.monitors();
    let monitor = monitors
        .item(0)
        .and_then(|object| object.downcast::<gdk::Monitor>().ok());
    let (width, height) = match monitor {
        Some(monitor) => {
            let geometry = monitor.geometry();
            (geometry.width(), geometry.height())
        }
        None => (1920, 1080),
    };
    let top = (height as f64 * TOP_RATIO) as i32;
    let side = ((width - WIDTH) / 2).max(0);
    (top, side)
}
