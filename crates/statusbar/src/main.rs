mod bar;
mod battery;
mod brightness;
mod control;
mod diagnose;
mod icons;
mod ipc;
mod logout;
mod network;
mod popup;
mod stats;
mod sysfs;
#[cfg(test)]
mod testing;
mod volume;

use gtk4::prelude::*;
use gtk4::Application;
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
    // Asking the bar what it can see is a read, not a run: it answers with the
    // pills this machine has and whether the icons paint, and it is worth having
    // when a bar is already up (which is exactly when something looks wrong).
    if std::env::args().skip(1).any(|arg| arg == "--diagnose") {
        print!("{}", diagnose::report());
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
    shell_common::theme::init(include_str!("../style.css"), "statusbar");

    let cfg = shell_common::config::get();
    let layer = LayerWindow::new(app);
    let on_top = cfg.statusbar.position != "bottom";
    layer.anchors(on_top, true, true, !on_top);
    layer.exclusive_zone(true);
    let height = cfg.statusbar.height + shell_common::theme::GAP;
    layer.window.set_height_request(height);

    let compositor = ipc::detect();
    let ui = bar::build(app, compositor);
    bar::start_tickers(&ui);
    if compositor != ipc::Compositor::Unknown {
        bar::follow(compositor, &ui.workspaces);
    }

    layer.window.set_child(Some(&ui.root));
    layer.window.present();
}
