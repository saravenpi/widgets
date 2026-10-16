//! The top-right toasts.
//!
//! The panel keeps the history, but a notification that arrives while the panel
//! is closed would otherwise be invisible until `mod+n`. These toasts surface it
//! where a notification daemon is expected to: a stack in the top-right, below
//! the statusbar, each one expiring on its own and dismissable with a click.
//! They are ephemeral and live in memory only — the state file the panel reads
//! is untouched by them.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use shell_common::layer::LayerWindow;
use shell_common::theme;

use crate::model::{self, Notification};

#[cfg(test)]
pub(crate) mod tests;

/// Seconds a toast stays on screen before it expires on its own.
const TOAST_SECONDS: i64 = 6;

/// The most toasts shown at once; older ones fall off the stack.
const MAX_TOASTS: usize = 4;

/// The toast width in logical pixels, the same measure the panel uses.
const TOAST_WIDTH: i32 = 360;

/// Close reasons from the desktop notification specification.
const REASON_EXPIRED: u32 = 1;
const REASON_DISMISSED: u32 = 2;

#[derive(Clone)]
struct Toast {
    notification: Notification,
    deadline: i64,
}

/// The stack of live toasts and the window that carries them.
pub struct Popups {
    window: gtk4::ApplicationWindow,
    list: gtk4::Box,
    toasts: RefCell<Vec<Toast>>,
    on_closed: Box<dyn Fn(u32, u32)>,
}

impl Popups {
    /// Build the toast surface for `application`. `on_closed` runs when a toast
    /// leaves the screen so the daemon can tell the client: reason 1 when it
    /// timed out, 2 when the user clicked it away.
    pub fn new(
        application: &gtk4::Application,
        on_closed: impl Fn(u32, u32) + 'static,
    ) -> Rc<Self> {
        let layer = LayerWindow::new(application).layer(Layer::Overlay);
        layer.anchors(true, false, true, false);
        layer.exclusive_zone(false);
        layer.keyboard_mode(KeyboardMode::None);
        layer.window.set_width_request(TOAST_WIDTH);
        layer.window.set_margin(Edge::Top, theme::WINDOW_TOP_GAP);
        layer.window.set_margin(Edge::Right, theme::GAP);

        let list = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        list.add_css_class("toasts");
        layer.window.set_child(Some(&list));

        let popups = Rc::new(Self {
            window: layer.window,
            list,
            toasts: RefCell::new(Vec::new()),
            on_closed: Box::new(on_closed),
        });

        let ticking = popups.clone();
        glib::timeout_add_seconds_local(1, move || {
            ticking.expire();
            glib::ControlFlow::Continue
        });

        popups
    }

    /// Show a notification as a toast, replacing an earlier toast with the same
    /// id (`replaces_id`) and dropping the oldest toast if the stack is full.
    pub fn add(self: &Rc<Self>, notification: Notification) {
        let deadline = model::now().saturating_add(lifetime(&notification));
        let mut toasts = self.toasts.borrow_mut();
        toasts.retain(|toast| toast.notification.id != notification.id);
        toasts.insert(0, Toast {
            notification,
            deadline,
        });
        toasts.truncate(MAX_TOASTS);
        drop(toasts);
        self.rebuild();
    }

    /// Remove a toast that left through the panel instead of the toast itself.
    pub fn dismiss(self: &Rc<Self>, id: u32) {
        let mut toasts = self.toasts.borrow_mut();
        let before = toasts.len();
        toasts.retain(|toast| toast.notification.id != id);
        let changed = toasts.len() != before;
        drop(toasts);
        if changed {
            self.rebuild();
        }
    }

    /// Drop every toast whose time is up. Runs once a second.
    pub(crate) fn expire(self: &Rc<Self>) {
        let now = model::now();
        let expired: Vec<u32> = {
            let toasts = self.toasts.borrow();
            toasts
                .iter()
                .filter(|toast| toast.deadline <= now)
                .map(|toast| toast.notification.id)
                .collect()
        };
        if expired.is_empty() {
            return;
        }
        self.toasts.borrow_mut().retain(|toast| toast.deadline > now);
        self.rebuild();
        for id in expired {
            (self.on_closed)(id, REASON_EXPIRED);
        }
    }

    /// Repaint the stack and show the surface only while it carries something.
    fn rebuild(self: &Rc<Self>) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let toasts = self.toasts.borrow().clone();
        for toast in &toasts {
            self.list.append(&self.row(toast));
        }
        if toasts.is_empty() {
            self.window.set_visible(false);
        } else {
            self.window.present();
        }
    }

    fn row(self: &Rc<Self>, toast: &Toast) -> gtk4::Box {
        let notification = &toast.notification;
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        row.add_css_class("toast");
        if notification.urgent {
            row.add_css_class("urgent");
        }

        row.append(&label(&notification.app_name, "dim", false));
        row.append(&label(&notification.summary, "summary", true));
        if !notification.body.is_empty() {
            row.append(&label(&notification.body, "body", true));
        }

        let click = gtk4::GestureClick::new();
        let this = self.clone();
        let id = notification.id;
        click.connect_released(move |_, _, _, _| {
            this.dismiss(id);
            (this.on_closed)(id, REASON_DISMISSED);
        });
        row.add_controller(click);

        row
    }
}

/// One line of a toast. The leading alignment and the wrap are the same on
/// every one of them, so only the text and the class differ — and the class is
/// what gives the line its place in the toast's type scale.
fn label(text: &str, class: &str, wrap: bool) -> gtk4::Label {
    let label = gtk4::Label::new(Some(text));
    label.add_css_class(class);
    label.set_halign(gtk4::Align::Start);
    label.set_xalign(0.0);
    label.set_wrap(wrap);
    label
}

/// How long a toast lives, in seconds. A critical notification stays until it
/// is dismissed, which is what a client sending `urgency == 3` expects.
fn lifetime(notification: &Notification) -> i64 {
    if notification.urgent {
        i64::MAX / 2
    } else {
        TOAST_SECONDS
    }
}
