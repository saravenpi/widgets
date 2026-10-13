use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

pub struct LayerWindow {
    pub window: gtk4::ApplicationWindow,
}

impl LayerWindow {
    pub fn new(app: &gtk4::Application) -> Self {
        let window = gtk4::ApplicationWindow::new(app);
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_namespace(Some("widgets"));
        LayerWindow { window }
    }

    pub fn anchors(&self, top: bool, left: bool, right: bool, bottom: bool) -> &Self {
        self.window.set_anchor(Edge::Top, top);
        self.window.set_anchor(Edge::Left, left);
        self.window.set_anchor(Edge::Right, right);
        self.window.set_anchor(Edge::Bottom, bottom);
        self
    }

    pub fn exclusive_zone(&self, exclusive: bool) -> &Self {
        if exclusive {
            self.window.auto_exclusive_zone_enable();
        } else {
            self.window.set_exclusive_zone(-1);
        }
        self
    }

    pub fn layer(self, layer: Layer) -> Self {
        self.window.set_layer(layer);
        self
    }

    pub fn keyboard_mode(&self, mode: gtk4_layer_shell::KeyboardMode) -> &Self {
        self.window.set_keyboard_mode(mode);
        self
    }

    pub fn present(&self) {
        self.window.present();
    }
}

pub fn ensure_supported() -> bool {
    if gtk4_layer_shell::is_supported() {
        return true;
    }
    eprintln!(
        "widgets: wlr-layer-shell is not supported on this display (Wayland compositors only)"
    );
    false
}