mod dbus;
mod model;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, glib, Application, ApplicationWindow, EventControllerKey};
use gtk4_layer_shell::{KeyboardMode, Layer};
use shell_common::layer::LayerWindow;

use dbus::Event;

thread_local! {
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

struct App {
    window: ApplicationWindow,
    list: gtk4::Box,
    scroll: gtk4::ScrolledWindow,
    count: gtk4::Label,
    empty: gtk4::Label,
    clear: gtk4::Button,
    items: RefCell<Vec<model::Notification>>,
    time_labels: RefCell<Vec<(gtk4::Label, i64)>>,
    handle: tokio::runtime::Handle,
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--version") => println!("sidebar 0.1.0"),
        Some(arg) if arg != "toggle" => {
            eprintln!("usage: sidebar [--version|toggle]");
            std::process::exit(2);
        }
        _ => toggle_or_start(),
    }
}

fn toggle_or_start() {
    if let Some(pid) = read_pid() {
        if process_alive(pid) && unsafe { libc::kill(pid, libc::SIGUSR1) } == 0 {
            std::process::exit(0);
        }
    }
    run_daemon();
}

fn run_daemon() {
    start_signal_listener();
    write_pidfile();
    let items = model::load();
    let next_id = items
        .iter()
        .map(|item| item.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let app = Application::builder()
        .application_id("dev.saravenpi.Sidebar")
        .build();
    app.connect_activate(move |app| activate(app, items.clone(), next_id));
    let code = app.run();
    remove_pidfile();
    std::process::exit(code.into());
}

fn activate(app: &Application, items: Vec<model::Notification>, next_id: u32) {
    if let Some(existing) = APP.with(|slot| slot.borrow().clone()) {
        existing.window.present();
        return;
    }
    if !shell_common::layer::ensure_supported() {
        remove_pidfile();
        std::process::exit(1);
    }
    shell_common::palette::apply_palette();
    shell_common::palette::apply_stylesheet(include_str!("style.css"));

    let layer = LayerWindow::new(app).layer(Layer::Overlay);
    layer.anchors(true, false, true, true);
    layer.exclusive_zone(false);
    layer.keyboard_mode(KeyboardMode::OnDemand);
    layer.window.set_width_request(360);

    let handle = dbus::start(next_id);
    let built = App::build(layer.window.clone(), items, handle);
    APP.with(|slot| *slot.borrow_mut() = Some(built));
}

fn start_signal_listener() {
    let mut signals = match signal_hook::iterator::Signals::new([libc::SIGUSR1]) {
        Ok(signals) => signals,
        Err(err) => {
            eprintln!("sidebar: cannot hook SIGUSR1: {err}");
            std::process::exit(1);
        }
    };
    std::thread::spawn(move || {
        for _ in signals.forever() {
            let _ = glib::idle_add_once(toggle_from_signal);
        }
    });
}

fn toggle_from_signal() {
    APP.with(|slot| {
        if let Some(app) = slot.borrow().as_ref() {
            app.toggle();
        }
    });
}

fn deliver(event: Event) {
    let _ = glib::idle_add_once(move || {
        APP.with(|slot| {
            if let Some(app) = slot.borrow().as_ref() {
                apply_event(&app, &event);
            }
        });
    });
}

fn apply_event(app: &Rc<App>, event: &Event) {
    match event {
        Event::Added(notification) => {
            model::insert(&mut app.items.borrow_mut(), notification.clone());
        }
        Event::Removed(id) => {
            app.items.borrow_mut().retain(|item| item.id != *id);
        }
    }
    model::save(&app.items.borrow());
    App::rebuild(app);
}

fn pidfile_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let mut path = PathBuf::from(dir);
        path.push("widgets-sidebar.pid");
        return path;
    }
    PathBuf::from("/tmp/widgets-sidebar.pid")
}

fn read_pid() -> Option<i32> {
    std::fs::read_to_string(pidfile_path())
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn process_alive(pid: i32) -> bool {
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn write_pidfile() {
    let _ = std::fs::write(pidfile_path(), std::process::id().to_string());
}

fn remove_pidfile() {
    if read_pid() == Some(std::process::id() as i32) {
        let _ = std::fs::remove_file(pidfile_path());
    }
}

impl App {
    fn build(
        window: ApplicationWindow,
        items: Vec<model::Notification>,
        handle: tokio::runtime::Handle,
    ) -> Rc<App> {
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header.add_css_class("header");

        let title = gtk4::Label::new(Some("Notifications"));
        title.add_css_class("title");
        title.set_halign(gtk4::Align::Start);
        title.set_hexpand(true);

        let count = gtk4::Label::new(Some("0"));
        count.add_css_class("count");

        let clear = gtk4::Button::with_label("Clear all");
        clear.add_css_class("clear");

        header.append(&title);
        header.append(&count);
        header.append(&clear);

        let list = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        list.add_css_class("list");

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_child(Some(&list));
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_vexpand(true);

        let empty = gtk4::Label::new(Some("No notifications"));
        empty.add_css_class("dim");

        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        root.add_css_class("panel");
        root.append(&header);
        root.append(&scroll);
        root.append(&empty);

        window.set_child(Some(&root));

        let app = Rc::new(App {
            window,
            list,
            scroll,
            count,
            empty,
            clear,
            items: RefCell::new(items),
            time_labels: RefCell::new(Vec::new()),
            handle,
        });

        let clear_app = app.clone();
        app.clear.connect_clicked(move |_| App::clear_all(&clear_app));

        let escape_window = app.window.clone();
        let key = EventControllerKey::new();
        key.connect_key_pressed(move |_, pressed, _, _| {
            if pressed == gdk::Key::Escape {
                escape_window.set_visible(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        app.window.add_controller(key);

        let tick_app = app.clone();
        glib::timeout_add_seconds_local(30, move || {
            tick_app.refresh_times();
            glib::ControlFlow::Continue
        });

        App::rebuild(&app);
        app
    }

    fn rebuild(app: &Rc<App>) {
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

        let summary = gtk4::Label::new(Some(&notification.summary));
        summary.add_css_class("summary");
        summary.set_halign(gtk4::Align::Start);
        summary.set_wrap(true);
        summary.set_xalign(0.0);

        row.append(&top);
        row.append(&summary);

        if !notification.body.is_empty() {
            let body = gtk4::Label::new(Some(&notification.body));
            body.add_css_class("body");
            body.set_halign(gtk4::Align::Start);
            body.set_wrap(true);
            body.set_xalign(0.0);
            row.append(&body);
        }

        row
    }

    fn dismiss(app: &Rc<App>, id: u32) {
        app.items.borrow_mut().retain(|item| item.id != id);
        model::save(&app.items.borrow());
        App::rebuild(app);
        dbus::emit_closed(&app.handle, id, 2);
    }

    fn clear_all(app: &Rc<App>) {
        let ids: Vec<u32> = app.items.borrow().iter().map(|item| item.id).collect();
        app.items.borrow_mut().clear();
        model::save(&app.items.borrow());
        App::rebuild(app);
        for id in ids {
            dbus::emit_closed(&app.handle, id, 2);
        }
    }

    fn refresh_times(&self) {
        let now = model::now();
        for (label, ts) in self.time_labels.borrow().iter() {
            label.set_text(&model::relative_time(*ts, now));
        }
    }

    fn toggle(&self) {
        if self.window.is_visible() {
            self.window.set_visible(false);
        } else {
            self.window.present();
        }
    }
}
