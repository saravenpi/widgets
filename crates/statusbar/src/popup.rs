//! A single shared popup window for the volume, brightness, battery and network controls.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Label, Orientation};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use shell_common::layer::LayerWindow;

/// The popup is a single overlay window reused by every control, paired with a
/// transparent full-screen backdrop that dismisses the card when clicked outside.
pub struct Popup {
    window: ApplicationWindow,
    backdrop: ApplicationWindow,
    root: Box,
    active_anchor: Rc<Cell<Option<usize>>>,
}

impl Popup {
    pub fn new(application: &Application) -> Self {
        let backdrop_layer = LayerWindow::new(application).layer(Layer::Top);
        backdrop_layer.anchors(true, true, true, true);
        backdrop_layer.exclusive_zone(false);
        backdrop_layer.keyboard_mode(KeyboardMode::None);
        let backdrop = backdrop_layer.window;
        backdrop.set_css_classes(&["backdrop"]);

        let layer = LayerWindow::new(application).layer(Layer::Overlay);
        layer.anchors(true, false, true, false);
        layer.exclusive_zone(false);
        layer.keyboard_mode(KeyboardMode::OnDemand);
        layer.window.set_margin(Edge::Top, shell_common::theme::WINDOW_TOP_GAP);
        layer.window.set_margin(Edge::Right, shell_common::theme::GAP);

        let window = layer.window;
        let root = Box::new(Orientation::Vertical, 0);
        root.add_css_class("popup");
        window.set_child(Some(&root));

        let active_anchor = Rc::new(Cell::new(None));

        let click = gtk4::GestureClick::new();
        let pop_win = window.clone();
        let back_win = backdrop.clone();
        let anchor_slot = active_anchor.clone();
        click.connect_pressed(move |_, _, _, _| {
            anchor_slot.set(None);
            pop_win.set_visible(false);
            back_win.set_visible(false);
        });
        backdrop.add_controller(click);

        let key = gtk4::EventControllerKey::new();
        let pop_win = window.clone();
        let back_win = backdrop.clone();
        let anchor_slot = active_anchor.clone();
        key.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gtk4::gdk::Key::Escape {
                anchor_slot.set(None);
                pop_win.set_visible(false);
                back_win.set_visible(false);
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        window.add_controller(key);

        let pop = Popup {
            window,
            backdrop,
            root,
            active_anchor,
        };
        pop.hide();
        pop
    }

    pub fn position_under(&self, anchor: &impl IsA<gtk4::Widget>) {
        self.active_anchor.set(Some(anchor.as_ptr() as usize));
        let Some(toplevel) = anchor.root() else {
            return;
        };
        let Some((x, _)) = anchor.translate_coordinates(&toplevel, 0.0, 0.0) else {
            return;
        };
        let right_edge = x + f64::from(anchor.allocated_width());
        let screen_width = f64::from(toplevel.allocated_width());
        let margin_right = (screen_width - right_edge).max(f64::from(shell_common::theme::GAP)) as i32;
        self.window.set_margin(Edge::Right, margin_right);
    }

    pub fn show<F: Fn(&Box)>(&self, build: F) {
        while let Some(child) = self.root.first_child() {
            self.root.remove(&child);
        }
        build(&self.root);
        self.backdrop.set_visible(true);
        self.backdrop.present();
        self.window.set_visible(true);
        self.window.present();
    }

    pub fn hide(&self) {
        self.active_anchor.set(None);
        self.window.set_visible(false);
        self.backdrop.set_visible(false);
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn is_open_for(&self, anchor: &impl IsA<gtk4::Widget>) -> bool {
        self.is_visible() && self.active_anchor.get() == Some(anchor.as_ptr() as usize)
    }
}

/// A popup holding a single title-and-value pair: what a control shows when its
/// backend cannot be read. Every control drew its own copy of this.
pub fn message(popup: &Popup, title: &str, value: &str) {
    popup.show(|pop| {
        let header = Label::new(Some(title));
        header.add_css_class("label");
        header.set_halign(gtk4::Align::Start);
        header.set_xalign(0.0);
        let value_label = Label::new(Some(value));
        value_label.add_css_class("value");
        value_label.set_halign(gtk4::Align::Start);
        value_label.set_xalign(0.0);
        pop.append(&header);
        pop.append(&value_label);
    });
}

/// A fill bar whose thumb fills `percent` of a 150 px track.
///
/// The volume, brightness and battery popups each drew their own copy; they share
/// this one so the three bars cannot drift apart.
pub fn fill_bar(percent: u16) -> Box {
    let bar = Box::new(Orientation::Horizontal, 0);
    bar.add_css_class("fill");
    let thumb = Box::new(Orientation::Vertical, 0);
    bar.append(&thumb);
    thumb.set_width_request((percent as f64 / 100.0 * 150.0) as i32);
    bar
}

impl Clone for Popup {
    fn clone(&self) -> Self {
        Popup {
            window: self.window.clone(),
            backdrop: self.backdrop.clone(),
            root: self.root.clone(),
            active_anchor: self.active_anchor.clone(),
        }
    }
}
