//! Watching the files a widget's stylesheet is built from.
//!
//! Two monitors can be live at once, and both end up in the thread-locals below,
//! which is what keeps them alive: a `FileMonitor` stops watching the moment it
//! is dropped, and nothing else holds a reference to these — the widgets live
//! for a whole session, so the slots live as long as the process does.

use std::cell::RefCell;

use gtk4::gio;
use gtk4::prelude::*;

use super::{notify_reload, rebuild};
use crate::palette;

thread_local! {
    static PALETTE_MONITOR: RefCell<Option<gio::FileMonitor>> = const { RefCell::new(None) };
    static STYLE_MONITOR: RefCell<Option<gio::FileMonitor>> = const { RefCell::new(None) };
}

/// Tie `provider` to the files the widget is styled from: the palette, and the
/// user's own sheet when they have placed one.
pub(super) fn watch(provider: gtk4::CssProvider, name: &str, default: &'static str) {
    if let Some(monitor) = watch_palette(&provider, name, default) {
        PALETTE_MONITOR.with(|slot| *slot.borrow_mut() = Some(monitor));
    }
    if let Some(monitor) = watch_user_sheet(&provider, name, default) {
        STYLE_MONITOR.with(|slot| *slot.borrow_mut() = Some(monitor));
    }
}

/// Watch the palette. A file that does not exist yet is watched too: the monitor
/// reports its creation, so a widget started before its first `palette sync`
/// picks the colours up when the file appears.
fn watch_palette(
    provider: &gtk4::CssProvider,
    name: &str,
    default: &'static str,
) -> Option<gio::FileMonitor> {
    let monitor = monitor_file(palette::palette_file()?)?;
    reload_on_change(&monitor, provider, name, default);
    Some(monitor)
}

/// Watch the user's own sheet at `$HOME/.config/widgets/<name>.css`. A file
/// that does not exist yet is watched too, exactly like the palette: the
/// monitor reports its creation, so a widget started before the user wrote
/// their sheet picks it up the moment it lands — a sheet placed after start-up
/// used to be ignored until the widget was restarted.
fn watch_user_sheet(
    provider: &gtk4::CssProvider,
    name: &str,
    default: &'static str,
) -> Option<gio::FileMonitor> {
    let path = palette::sheet_file(name)?;
    let monitor = monitor_file(path)?;
    reload_on_change(&monitor, provider, name, default);
    Some(monitor)
}

fn monitor_file(path: std::path::PathBuf) -> Option<gio::FileMonitor> {
    gio::File::for_path(path)
        .monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
        .ok()
}

/// Reload: build the document from the files as they are now, load it back into
/// `provider`, and tell the listeners it moved.
///
/// Reading the files again is the whole point. A document captured when the
/// widget started would be loaded back unchanged on every change, so `palette
/// sync` would have no effect on a bar that is already up.
pub(super) fn reload(provider: &gtk4::CssProvider, name: &str, default: &'static str) {
    provider.load_from_data(&rebuild(name, default));
    notify_reload();
}

/// Wire a monitor to [`reload`].
fn reload_on_change(
    monitor: &gio::FileMonitor,
    provider: &gtk4::CssProvider,
    name: &str,
    default: &'static str,
) {
    let provider = provider.clone();
    let name = name.to_string();
    monitor.connect_changed(move |_, _, _, _| reload(&provider, &name, default));
}
