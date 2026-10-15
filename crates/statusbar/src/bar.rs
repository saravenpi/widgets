//! The bar itself: the modules on it, the timers that keep them current, and the
//! workspace chips that follow the compositor.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, Box, CenterBox, Label};

use shell_common::theme::MODULE_GAP;

use crate::ipc;
use crate::popup;
use crate::{control, icons, stats};

/// Everything on the bar that has to outlive the function that builds it. The
/// label fields are the ones a ticker reaches back into.
pub struct Ui {
    pub root: CenterBox,
    pub workspaces: Box,
    clock: Label,
    ram: Label,
    cpu: Label,
    battery_label: Option<Label>,
    network_label: Option<Label>,
}

/// Build the bar: the workspace strip, the clock, the CPU/RAM readings and the
/// control row, in that order.
///
/// The three are the bar's modules: workspaces on the left, the clock in the
/// middle, the CPU and memory pills and the control row on the right. A centre
/// box rather than a row of expanding children is what actually holds the clock
/// in the middle of the bar — three children sharing the spare width put it
/// wherever the wider end left it, a good 100 px off centre — and one
/// [`MODULE_GAP`] separates every pair of modules and every pair of items
/// inside one.
pub fn build(app: &Application, compositor: ipc::Compositor) -> Ui {
    let root = CenterBox::new();
    root.add_css_class("bar");

    let workspaces = Box::new(gtk4::Orientation::Horizontal, MODULE_GAP);
    workspaces.add_css_class("workspaces");
    workspaces.set_halign(gtk4::Align::Start);
    if compositor == ipc::Compositor::Unknown {
        workspaces.set_visible(false);
    }

    let clock = Label::new(None);
    clock.add_css_class("clock");
    clock.set_halign(gtk4::Align::Center);

    // The pills collect their glyphs in here, and the hook holds the same set:
    // the icons are drawn rather than styled, so they cannot follow the cascade
    // the way the labels do, and this is what repaints them every time the
    // stylesheet is rebuilt.
    let painted = Rc::new(RefCell::new(icons::Painted::default()));
    icons::follow_palette(painted.clone());
    let controls = control::build(&popup::Popup::new(app), &painted);
    let (right, ram, cpu) = right_row(&controls.container);

    root.set_start_widget(Some(&workspaces));
    root.set_center_widget(Some(&clock));
    root.set_end_widget(Some(&right));

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

/// The bar's right-hand module: the CPU and memory readings as two pills, then
/// the control row, one gap apart — with the two labels a ticker refreshes.
///
/// They are one piece rather than two. Appended to the bar one after the other
/// they had no gap between them at all, so the RAM pill sat flush against the
/// volume pill while every other pair on the bar kept its own gap.
fn right_row(controls: &Box) -> (Box, Label, Label) {
    let ram = Label::new(None);
    ram.add_css_class("stat");
    let cpu = Label::new(None);
    cpu.add_css_class("stat");

    let stats = Box::new(gtk4::Orientation::Horizontal, MODULE_GAP);
    stats.add_css_class("stats");
    stats.append(&ram);
    stats.append(&cpu);

    let right = Box::new(gtk4::Orientation::Horizontal, MODULE_GAP);
    right.add_css_class("right");
    right.set_halign(gtk4::Align::End);
    right.append(&stats);
    right.append(controls);
    (right, ram, cpu)
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
