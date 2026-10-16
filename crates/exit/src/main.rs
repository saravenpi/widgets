//! `exit`: desktop session exit dialog.

use std::process::Command;

use gtk4::prelude::*;
use gtk4::{gdk, Application, Box, Button, Label, Orientation};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use shell_common::app::run as run_app;
use shell_common::layer::{ensure_supported, LayerWindow};
use shell_common::theme;

const STYLE: &str = include_str!("../style.css");
const WIDTH: i32 = 440;

fn main() {
    if std::env::args().skip(1).any(|argument| argument == "--version") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }
    let application = Application::builder()
        .application_id("dev.saravenpi.Exit")
        .build();
    application.connect_activate(activate);
    let _ = run_app(&application, env!("CARGO_PKG_NAME"));
}

fn activate(application: &Application) {
    if !ensure_supported() {
        std::process::exit(1);
    }
    theme::init(STYLE, "exit");

    let exit_layer = layer_window(application);
    let window = exit_layer.window;

    let escape_key = gtk4::EventControllerKey::new();
    let escape_window = window.clone();
    escape_key.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gdk::Key::Escape {
            escape_window.close();
            return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    window.add_controller(escape_key);

    let surface = Box::new(Orientation::Vertical, 0);
    surface.add_css_class("surface");
    surface.add_css_class("exit-dialog");
    surface.set_width_request(WIDTH);

    let title = Label::new(Some("Exit Sway"));
    title.add_css_class("title");
    title.set_halign(gtk4::Align::Start);
    surface.append(&title);

    let subtitle = Label::new(Some("End desktop session, reboot, or shut down"));
    subtitle.add_css_class("subtitle");
    subtitle.set_halign(gtk4::Align::Start);
    surface.append(&subtitle);

    let actions = Box::new(Orientation::Horizontal, 8);
    actions.add_css_class("actions");
    actions.set_halign(gtk4::Align::End);

    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.add_css_class("action");
    cancel_btn.add_css_class("cancel");
    let cancel_win = window.clone();
    cancel_btn.connect_clicked(move |_| {
        cancel_win.close();
    });

    let exit_btn = Button::with_label("Exit");
    exit_btn.add_css_class("action");
    exit_btn.add_css_class("exit");
    let exit_win = window.clone();
    exit_btn.connect_clicked(move |_| {
        exit_win.close();
        let _ = Command::new("swaymsg").arg("exit").spawn().or_else(|_| {
            let sid = std::env::var("XDG_SESSION_ID").unwrap_or_default();
            Command::new("loginctl")
                .arg("terminate-session")
                .arg(sid)
                .spawn()
        });
    });

    let reboot_btn = Button::with_label("Reboot");
    reboot_btn.add_css_class("action");
    reboot_btn.add_css_class("reboot");
    let reboot_win = window.clone();
    reboot_btn.connect_clicked(move |_| {
        reboot_win.close();
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });

    let power_btn = Button::with_label("Power Off");
    power_btn.add_css_class("action");
    power_btn.add_css_class("poweroff");
    let power_win = window.clone();
    power_btn.connect_clicked(move |_| {
        power_win.close();
        let _ = Command::new("systemctl").arg("poweroff").spawn();
    });

    actions.append(&cancel_btn);
    actions.append(&exit_btn);
    actions.append(&reboot_btn);
    actions.append(&power_btn);
    surface.append(&actions);

    window.set_child(Some(&surface));
    window.present();
    cancel_btn.grab_focus();
}

fn layer_window(application: &Application) -> LayerWindow {
    let layer = LayerWindow::new(application).layer(Layer::Overlay);
    layer.anchors(true, true, true, false);
    layer.exclusive_zone(false);
    layer.keyboard_mode(KeyboardMode::Exclusive);
    let (top, side) = margins();
    layer.window.set_margin(Edge::Top, top);
    layer.window.set_margin(Edge::Left, side);
    layer.window.set_margin(Edge::Right, side);
    layer
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
    let top = ((height - 160) / 2).max(0);
    let side = ((width - WIDTH) / 2).max(0);
    (top, side)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_sheet_loads_cleanly() {
        assert!(!STYLE.is_empty());
        assert!(STYLE.contains(".exit-dialog"));
        assert!(STYLE.contains("button.action"));
    }
}
