//! The bar itself: the modules on it, the timers that keep them current, and the
//! workspace chips that follow the compositor.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, Box, Label};

use crate::ipc;
use crate::popup;
use crate::{control, stats};

/// Everything on the bar that has to outlive the function that builds it. The
/// label fields are the ones a ticker reaches back into.
pub struct Ui {
    pub root: Box,
    pub workspaces: Box,
    clock: Label,
    ram: Label,
    cpu: Label,
    battery_label: Option<Label>,
    network_label: Option<Label>,
}

/// Build the bar: the workspace strip, the clock, the CPU/RAM readings and the
/// control row, in that order.
pub fn build(app: &Application, compositor: ipc::Compositor) -> Ui {
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

    let (stats_row, ram, cpu) = stats_row();
    let controls = control::build(&popup::Popup::new(app));

    root.append(&workspaces);
    root.append(&clock);
    root.append(&stats_row);
    root.append(&controls.container);

    Ui {
        root,
        workspaces,
        clock,
        ram,
        cpu,
        battery_label: controls.battery_label,
        network_label: controls.network_label,
    }
}

/// The CPU and memory readings, as two pills at the right-hand end, so both ends
/// of the bar read as one piece.
fn stats_row() -> (Box, Label, Label) {
    let ram = Label::new(None);
    ram.add_css_class("stat");
    let cpu = Label::new(None);
    cpu.add_css_class("stat");
    let row = Box::new(gtk4::Orientation::Horizontal, 6);
    row.add_css_class("stats");
    row.set_hexpand(true);
    row.set_halign(gtk4::Align::End);
    row.append(&ram);
    row.append(&cpu);
    (row, ram, cpu)
}

/// Put the clock, the CPU/RAM readings and the battery and network pills on their
/// own timers. Only battery and network tick: volume and brightness update when
/// they are clicked, because for those the popup is the source of truth.
pub fn start_tickers(ui: &Ui) {
    let battery = ui.battery_label.clone();
    glib::timeout_add_seconds_local(30, move || {
        if let Some(ref label) = battery {
            crate::battery::refresh(label);
        }
        glib::ControlFlow::Continue
    });

    let network = ui.network_label.clone();
    glib::timeout_add_seconds_local(15, move || {
        if let Some(ref label) = network {
            crate::network::refresh(label);
        }
        glib::ControlFlow::Continue
    });

    update_clock(&ui.clock);
    let clock = ui.clock.clone();
    glib::timeout_add_seconds_local(30, move || {
        update_clock(&clock);
        glib::ControlFlow::Continue
    });

    // The CPU share is a delta, so the first reading only sets the baseline;
    // every tick after it reports the busy share since the previous one.
    let ram = ui.ram.clone();
    let cpu = ui.cpu.clone();
    let mut sampler = stats::Sampler::new();
    update_stats(&ram, &cpu, &mut sampler);
    glib::timeout_add_seconds_local(2, move || {
        update_stats(&ram, &cpu, &mut sampler);
        glib::ControlFlow::Continue
    });
}

/// Subscribe to the compositor's workspace events and redraw the chips to match.
pub fn follow(compositor: ipc::Compositor, container: &Box) {
    let (tx, rx) = async_channel::unbounded::<Vec<ipc::Workspace>>();
    let container = container.clone();
    glib::spawn_future_local(async move {
        while let Ok(state) = rx.recv().await {
            rebuild(&container, &state, compositor);
        }
    });
    ipc::start(compositor, tx);
}

fn rebuild(container: &Box, state: &[ipc::Workspace], compositor: ipc::Compositor) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    for workspace in state {
        container.append(&chip(workspace, compositor));
    }
}

/// One workspace chip: its label, whether it is focused or urgent, and a click
/// that asks the compositor to focus it.
fn chip(workspace: &ipc::Workspace, compositor: ipc::Compositor) -> gtk4::Button {
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
    button
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
