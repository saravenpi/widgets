//! Drawing the panel's rows from the items it holds.

use std::rc::Rc;

use gtk4::prelude::*;

use crate::model;

use super::App;

impl App {
    /// Redraw the list from `items`.
    ///
    /// The time labels are refreshed on a timer rather than redrawn, so they are
    /// collected here on the way past and handed back to the panel.
    pub(super) fn rebuild(app: &Rc<App>) {
        let mut times: Vec<(gtk4::Label, i64)> = Vec::new();
        while let Some(child) = app.list.first_child() {
            app.list.remove(&child);
        }
        let now = model::now();
        for notification in app.items.borrow().iter() {
            let row = App::row(app, notification, now, &mut times);
            app.list.append(&row);
        }
        *app.time_labels.borrow_mut() = times;

        let total = app.items.borrow().len();
        app.count.set_text(&total.to_string());
        app.clear.set_sensitive(total > 0);
        app.scroll.set_visible(total > 0);
        app.empty.set_visible(total == 0);
    }

    /// One notification row: the app and its age, the summary, and the body when
    /// the notification carried one.
    fn row(
        app: &Rc<App>,
        notification: &model::Notification,
        now: i64,
        times: &mut Vec<(gtk4::Label, i64)>,
    ) -> gtk4::Box {
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        row.add_css_class("row");
        if notification.urgent {
            row.add_css_class("urgent");
        }

        row.append(&App::row_top(app, notification, now, times));

        if let Some(body) = row_body(notification) {
            row.append(&body);
        }
        row
    }

    /// The top line of a row: the app name, its age, and the dismiss button.
    fn row_top(
        app: &Rc<App>,
        notification: &model::Notification,
        now: i64,
        times: &mut Vec<(gtk4::Label, i64)>,
    ) -> gtk4::Box {
        let top = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

        let name = gtk4::Label::new(Some(&notification.app_name));
        name.add_css_class("dim");
        name.set_halign(gtk4::Align::Start);
        name.set_hexpand(true);

        let time = gtk4::Label::new(Some(&model::relative_time(notification.ts, now)));
        time.add_css_class("dim");
        times.push((time.clone(), notification.ts));

        let dismiss = gtk4::Button::with_label("x");
        dismiss.add_css_class("dismiss");
        let row_app = app.clone();
        let row_id = notification.id;
        dismiss.connect_clicked(move |_| App::dismiss(&row_app, row_id));

        top.append(&name);
        top.append(&time);
        top.append(&dismiss);
        top
    }
}

/// The body line, when there is one. An empty body is left out rather than shown
/// as a blank line.
fn row_body(notification: &model::Notification) -> Option<gtk4::Label> {
    if notification.body.is_empty() {
        return None;
    }
    let body = gtk4::Label::new(Some(&notification.body));
    body.add_css_class("body");
    body.set_halign(gtk4::Align::Start);
    body.set_wrap(true);
    body.set_xalign(0.0);
    Some(body)
}
