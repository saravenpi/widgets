mod ipc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, Label};
use shell_common::app::run as run_app;
use shell_common::layer::{ensure_supported, LayerWindow};

/// The name this widget claims in `$XDG_RUNTIME_DIR` while the bar lives.
const WIDGET: &str = "statusbar";

fn main() {
    if std::env::args().skip(1).any(|arg| arg == "--version") {
        println!(
            "{} {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
        return;
    }
    // The bar is a singleton: a compositor that runs it twice (a reload, a
    // stray `exec`) would otherwise stack two bars in the same exclusive zone.
    if let Some(pid) = shell_common::instance::claim(WIDGET) {
        eprintln!("statusbar: already running (pid {pid}), leaving it alone");
        return;
    }
    let app = Application::builder()
        .application_id("dev.saravenpi.Statusbar")
        .build();
    app.connect_activate(activate);
    std::process::exit(run_app(&app, env!("CARGO_PKG_NAME")).into());
}

fn activate(app: &Application) {
    if !ensure_supported() {
        std::process::exit(1);
    }
    shell_common::theme::init(include_str!("../style.css"));

    let bar = LayerWindow::new(app);
    bar.anchors(true, true, true, false);
    bar.exclusive_zone(true);
    bar.window.set_height_request(shell_common::theme::BAR_HEIGHT);

    let compositor = ipc::detect();
    let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    root.add_css_class("bar");

    let workspaces = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    workspaces.add_css_class("workspaces");
    workspaces.set_hexpand(true);
    workspaces.set_halign(gtk4::Align::Start);
    if compositor == ipc::Compositor::Unknown {
        workspaces.set_visible(false);
    }

    let clock = Label::new(None);
    clock.add_css_class("clock");
    clock.set_hexpand(true);
    clock.set_halign(gtk4::Align::Center);

    let badge = Label::new(Some(ipc::name(compositor)));
    badge.add_css_class("badge");
    badge.add_css_class("dim");
    badge.set_hexpand(true);
    badge.set_halign(gtk4::Align::End);

    root.append(&workspaces);
    root.append(&clock);
    root.append(&badge);
    bar.window.set_child(Some(&root));

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

    bar.window.present();
}


fn rebuild(container: &gtk4::Box, state: &[ipc::Workspace], compositor: ipc::Compositor) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    for workspace in state {
        let button = gtk4::Button::with_label(&workspace.label);
        button.add_css_class("ws");
        button.set_valign(gtk4::Align::Center);
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
