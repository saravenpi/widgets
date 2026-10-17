//! `settings`: desktop settings and display manager widget.

mod displays;
mod pages;
mod sway;
mod ui;

use gtk4::prelude::*;
use gtk4::{gdk, Application, Box, Orientation};
use gtk4_layer_shell::{KeyboardMode, Layer};
use shell_common::app::run as run_app;
use shell_common::layer::{ensure_supported, LayerWindow};
use shell_common::theme;

const STYLE: &str = include_str!("../style.css");

fn main() {
    if std::env::args().skip(1).any(|arg| arg == "--version") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }
    let application = Application::builder()
        .application_id("dev.saravenpi.Settings")
        .build();
    application.connect_activate(activate);
    let _ = run_app(&application, env!("CARGO_PKG_NAME"));
}

fn activate(application: &Application) {
    if !ensure_supported() {
        std::process::exit(1);
    }
    theme::init(STYLE, "settings");

    let layer = LayerWindow::new(application).layer(Layer::Overlay);
    layer.anchors(true, true, true, true);
    layer.exclusive_zone(false);
    layer.keyboard_mode(KeyboardMode::Exclusive);
    layer.window.set_css_classes(&["settings-overlay"]);
    let window = layer.window;

    let escape_key = gtk4::EventControllerKey::new();
    let escape_win = window.clone();
    escape_key.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gdk::Key::Escape {
            escape_win.close();
            return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    window.add_controller(escape_key);

    let surface = ui::build_surface(&window);

    let center = Box::new(Orientation::Vertical, 0);
    center.set_valign(gtk4::Align::Center);
    center.set_halign(gtk4::Align::Center);
    center.append(&surface);

    let click = gtk4::GestureClick::new();
    let backdrop_win = window.clone();
    let dialog_box = surface.clone();
    click.connect_pressed(move |_, _, x, y| {
        let (surface_x, surface_y) = match dialog_box.translate_coordinates(&backdrop_win, 0.0, 0.0) {
            Some(coords) => coords,
            None => return,
        };
        let width = f64::from(dialog_box.allocated_width());
        let height = f64::from(dialog_box.allocated_height());
        let inside = x >= surface_x && x <= surface_x + width && y >= surface_y && y <= surface_y + height;
        if !inside {
            backdrop_win.close();
        }
    });
    window.add_controller(click);

    window.set_child(Some(&center));
    window.present();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_sheet_loads_cleanly() {
        assert!(!STYLE.is_empty());
        assert!(STYLE.contains(".settings-dialog"));
        assert!(STYLE.contains("button.action"));
    }
}
