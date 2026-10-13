mod ipc;

use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Label};
use gtk4_layer_shell::LayerShell;

fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!("statusbar 0.1.0");
        return;
    }
    let app = Application::builder()
        .application_id("dev.saravenpi.Statusbar")
        .build();
    app.connect_activate(activate);
    std::process::exit(app.run().into());
}

fn activate(app: &Application) {
    if !gtk4_layer_shell::is_supported() {
        eprintln!(
            "statusbar: gtk4-layer-shell is not supported on this display; \
             a Wayland compositor with zwlr_layer_shell_v1 is required"
        );
        std::process::exit(1);
    }
    let window = ApplicationWindow::builder().application(app).build();
    window.init_layer_shell();
    window.set_layer(gtk4_layer_shell::Layer::Top);
    window.set_anchor(gtk4_layer_shell::Edge::Top, true);
    window.set_anchor(gtk4_layer_shell::Edge::Left, true);
    window.set_anchor(gtk4_layer_shell::Edge::Right, true);
    window.auto_exclusive_zone_enable();
    window.set_height_request(30);

    shell_common::palette::apply_palette();
    shell_common::palette::apply_stylesheet(include_str!("../style.css"));

    let compositor = ipc::detect();
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("bar");

    let workspaces = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    workspaces.add_css_class("workspaces");
    workspaces.set_hexpand(true);
    workspaces.set_halign(gtk::Align::Start);
    if compositor == ipc::Compositor::Unknown {
        workspaces.set_visible(false);
    }

    let clock = Label::new(None);
    clock.add_css_class("clock");
    clock.set_hexpand(true);
    clock.set_halign(gtk::Align::Center);

    let badge = Label::new(Some(ipc::name(compositor)));
    badge.add_css_class("badge");
    badge.set_hexpand(true);
    badge.set_halign(gtk::Align::End);

    root.append(&workspaces);
    root.append(&clock);
    root.append(&badge);
    window.set_child(Some(&root));

    update_clock(&clock);
    let ticking_clock = clock.clone();
    glib::timeout_add_seconds_local(30, move || {
        update_clock(&ticking_clock);
        glib::ControlFlow::Continue
    });

    if compositor != ipc::Compositor::Unknown {
        let (tx, rx) = async_channel::unbounded::<Vec<ipc::Workspace>>();
        let container = workspaces.clone();
        glib::spawn_future_local(async move {
            while let Ok(state) = rx.recv().await {
                rebuild(&container, &state, compositor);
            }
        });
        ipc::start(compositor, tx);
    }

    window.present();
}

fn rebuild(container: &gtk::Box, state: &[ipc::Workspace], compositor: ipc::Compositor) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    for workspace in state {
        let button = gtk::Button::with_label(&workspace.label);
        button.add_css_class("ws");
        button.set_valign(gtk::Align::Center);
        if workspace.focused {
            button.add_css_class("focused");
        }
        if workspace.urgent {
            button.add_css_class("urgent");
        }
        let key = workspace.key.clone();
        button.connect_clicked(move |_| ipc::focus(compositor, &key));
        container.append(&button);
    }
}

fn update_clock(label: &Label) {
    label.set_text(&chrono::Local::now().format("%H:%M").to_string());
}
