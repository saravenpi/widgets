mod battery;
mod brightness;
mod ipc;
mod logout;
mod network;
mod popup;
mod stats;
mod volume;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, Box, Label};
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
    shell_common::theme::init(include_str!("../style.css"), "statusbar");

    let bar = LayerWindow::new(app);
    bar.anchors(true, true, true, false);
    bar.exclusive_zone(true);
    bar.window.set_height_request(shell_common::theme::BAR_HEIGHT);

    let compositor = ipc::detect();
    let root = Box::new(gtk4::Orientation::Horizontal, 0);
    root.add_css_class("bar");

    let workspaces = Box::new(gtk4::Orientation::Horizontal, 6);
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

    // CPU and memory, the same soft pills as the workspace chips, so both ends
    // of the bar read as one piece.
    let ram = Label::new(None);
    ram.add_css_class("stat");
    let cpu = Label::new(None);
    cpu.add_css_class("stat");
    let stats = Box::new(gtk4::Orientation::Horizontal, 6);
    stats.add_css_class("stats");
    stats.set_hexpand(true);
    stats.set_halign(gtk4::Align::End);
    stats.append(&ram);
    stats.append(&cpu);

    // Controls on the right, after the stats: volume, brightness, battery,
    // network, logout. Each is a soft pill matching the stat pills; the last one
    // (logout) is the only one that carries the urgency colour so it reads as an
    // action.
    let controls = Box::new(gtk4::Orientation::Horizontal, 6);
    controls.add_css_class("controls");
    controls.set_hexpand(false);
    controls.set_halign(gtk4::Align::End);

    let popup = popup::Popup::new(app);

    // Battery and network labels must outlive their tickers, so keep them in
    // variables and tick them on intervals. Volume and brightness update on click
    // only (the popup is the source of truth there).
    let mut bat_label: Option<Label> = None;
    let mut net_label: Option<Label> = None;

    if volume::available() {
        let vol_label = Label::new(Some(&format!("Vol {}", volume::percent().unwrap_or(0))));
        vol_label.add_css_class("pill");
        let vol_button = volume::pill(popup.clone(), vol_label);
        controls.append(&vol_button);
    }

    if brightness::available() {
        let bright_label = Label::new(Some(&format!(
            "Brightness {}",
            brightness::percent().unwrap_or(0)
        )));
        bright_label.add_css_class("pill");
        let bright_button = brightness::pill(popup.clone(), bright_label);
        controls.append(&bright_button);
    }

    if battery::available() {
        let bat_label_inner = Label::new(Some(&format!(
            "Bat {pct}{st}",
            pct = battery::percent().unwrap_or(0),
            st = battery::state_label().unwrap_or(""),
        )));
        bat_label_inner.add_css_class("pill");
        bat_label = Some(bat_label_inner.clone());
        let bat_button = battery::pill(popup.clone(), bat_label_inner);
        controls.append(&bat_button);
    }

    if network::available() {
        let net_text = network::label_text().unwrap_or_else(|| "Net ".to_string());
        let net_label_inner = Label::new(Some(&net_text));
        net_label_inner.add_css_class("pill");
        net_label = Some(net_label_inner.clone());
        let net_button = network::pill(popup.clone(), net_label_inner);
        controls.append(&net_button);
    }

    if let Some(logout_button) = logout::pill() {
        controls.append(&logout_button);
    }

    let bat_label = bat_label; // move into the ticker closure
    glib::timeout_add_seconds_local(30, move || {
        if let Some(ref label) = bat_label {
            battery::refresh(label);
        }
        glib::ControlFlow::Continue
    });
    let net_label = net_label; // move into the ticker closure
    glib::timeout_add_seconds_local(15, move || {
        if let Some(ref label) = net_label {
            network::refresh(label);
        }
        glib::ControlFlow::Continue
    });

    root.append(&workspaces);
    root.append(&clock);
    root.append(&stats);
    root.append(&controls);
    bar.window.set_child(Some(&root));

    update_clock(&clock);
    let ticking_clock = clock.clone();
    glib::timeout_add_seconds_local(30, move || {
        update_clock(&ticking_clock);
        glib::ControlFlow::Continue
    });

    // The CPU share is a delta, so the first reading only sets the baseline;
    // every tick after it reports the busy share since the previous one.
    let mut sampler = stats::Sampler::new();
    update_stats(&ram, &cpu, &mut sampler);
    glib::timeout_add_seconds_local(2, move || {
        update_stats(&ram, &cpu, &mut sampler);
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


fn rebuild(container: &Box, state: &[ipc::Workspace], compositor: ipc::Compositor) {
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

fn update_stats(ram: &Label, cpu: &Label, sampler: &mut stats::Sampler) {
    if let Some(percent) = stats::memory_percent() {
        ram.set_text(&format!("RAM {percent}%"));
    }
    if let Some(percent) = sampler.cpu_percent() {
        cpu.set_text(&format!("CPU {percent}%"));
    }
}
