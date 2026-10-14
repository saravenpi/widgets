//! What the panel does: drop a notification, empty itself, keep its relative
//! times current, and show or hide.

use std::rc::Rc;

use gtk4::prelude::*;

use crate::{dbus, model};

use super::App;

impl App {
    /// Drop one notification, from the history file, the list and any toast.
    pub(super) fn dismiss(app: &Rc<App>, id: u32) {
        app.items.borrow_mut().retain(|item| item.id != id);
        model::save(&app.items.borrow());
        App::rebuild(app);
        app.toasts.dismiss(id);
        dbus::emit_closed(&app.handle, id, 2);
    }

    /// Empty the panel, telling each client the notification was closed.
    pub(super) fn clear_all(app: &Rc<App>) {
        let ids: Vec<u32> = app.items.borrow().iter().map(|item| item.id).collect();
        app.items.borrow_mut().clear();
        model::save(&app.items.borrow());
        App::rebuild(app);
        for id in ids {
            app.toasts.dismiss(id);
            dbus::emit_closed(&app.handle, id, 2);
        }
    }

    pub(super) fn refresh_times(&self) {
        let now = model::now();
        for (label, ts) in self.time_labels.borrow().iter() {
            label.set_text(&model::relative_time(*ts, now));
        }
    }

    pub(crate) fn toggle(&self) {
        if self.window.is_visible() {
            self.window.set_visible(false);
        } else {
            self.window.present();
        }
    }
}
