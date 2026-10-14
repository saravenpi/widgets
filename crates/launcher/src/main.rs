//! `launcher`: the application palette.
//!
//! This file is the entry point and the window placement. The surface itself —
//! the query field, the results, the keys — is [`popup`].

mod apps;
mod exec;
mod popup;

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, Application, Box, Orientation};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use shell_common::app::run as run_app;
use shell_common::layer::{ensure_supported, LayerWindow};
use shell_common::theme;

use apps::App;

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
    theme::init(STYLE, "launcher");

    let launcher = layer_window(application);
    let window = launcher.window.clone();
    let (entry, list) = popup::build();
    popup::install(&window, &entry, &list, apps);

    let surface = Box::new(Orientation::Vertical, 0);
    surface.add_css_class("surface");
    surface.add_css_class("popup");
    surface.set_width_request(WIDTH);
    surface.append(&entry);
    surface.append(&list);
    window.set_child(Some(&surface));

    popup::fill(&list, apps, "");
    window.present();
    entry.grab_focus();
}

/// The overlay the launcher floats on: full width under the top edge, with the
/// field a third of the way down rather than flush against the statusbar.
fn layer_window(application: &Application) -> LayerWindow {
    let launcher = LayerWindow::new(application).layer(Layer::Overlay);
    launcher.anchors(true, true, true, false);
    launcher.exclusive_zone(false);
    launcher.keyboard_mode(KeyboardMode::Exclusive);
    let (top, side) = margins();
    launcher.window.set_margin(Edge::Top, top);
    launcher.window.set_margin(Edge::Left, side);
    launcher.window.set_margin(Edge::Right, side);
    launcher
}

/// Where the field sits: a third down the monitor, and the width it keeps in
/// the middle of it. Without a monitor to ask, the placeholders are 1920x1080.
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
