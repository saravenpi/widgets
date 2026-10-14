//! A single shared popup window for the volume and brightness controls.

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Orientation};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use shell_common::layer::LayerWindow;

/// The popup is a single overlay window reused by every control. Each control
/// clears it and repopulates it, so there is only ever one popup on screen at a
/// time and the bar never stacks two cards.
///
/// The window and its content box are GTK GObjects (reference counted), so they
/// are `Clone` and every pill can hold its own `Popup` that shares the same
/// underlying surface.
pub struct Popup {
    window: ApplicationWindow,
    root: Box,
}

impl Popup {
    pub fn new(application: &Application) -> Self {
        let layer = LayerWindow::new(application).layer(Layer::Top);
        layer.anchors(true, true, false, false);
        layer.window.set_margin(Edge::Top, shell_common::theme::BAR_HEIGHT);
        layer.window.set_margin(Edge::Left, shell_common::theme::GAP);
        layer.window.set_margin(Edge::Right, shell_common::theme::GAP);

        let window = layer.window;
        let root = Box::new(Orientation::Vertical, 0);
        root.add_css_class("popup");
        window.set_child(Some(&root));

        let pop = Popup { window, root };
        pop.hide();
        pop
    }

    pub fn show<F: Fn(&Box)>(&self, build: F) {
        while let Some(child) = self.root.first_child() {
            self.root.remove(&child);
        }
        build(&self.root);
        self.window.set_visible(true);
        self.window.present();
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

}

impl Clone for Popup {
    fn clone(&self) -> Self {
        Popup {
            window: self.window.clone(),
            root: self.root.clone(),
        }
    }
}

