//! The shared visual language of every widget.
//!
//! One base stylesheet defines what all three widgets agree on — transparent
//! windows, muted text, soft pill buttons, panel surfaces — so a widget's own
//! stylesheet only has to say what makes it different. Colours come from the
//! palette file (`~/.config/widgets/palette.css`), never from literals.
//!
//! [`init`] loads the three layers into a single provider and then watches the
//! palette file. `palette sync` rewrites that file whenever the wallpaper or the
//! palette changes, so a running widget restyles itself instead of waiting for a
//! restart.

use std::cell::RefCell;

use gtk4::prelude::*;

use crate::palette;

/// The gap around a floating widget and the window rectangles it sits inside,
/// matching `gaps outer` in the compositor config. The bar is inset by it on
/// every side, and the sidebar and its toasts keep it off the right and bottom
/// edges so they float the same way.
pub const GAP: i32 = 10;
/// `gaps inner` in the compositor config, the same measure used between the bar
/// and the windows it sits beside.
pub const GAP_INNER: i32 = 10;

/// Height of the statusbar, and the strip it reserves at the top of the
/// screen. A widget that docks to an edge keeps its content out of that strip
/// so it never sits underneath the bar. The bar itself is a floating pill, so
/// this is the pill (32px) plus a `GAP` above and below it.
pub const BAR_HEIGHT: i32 = 32 + GAP * 2;

/// Loaded before every widget stylesheet, which may override any of it.
///
/// The rules here follow one strict scale so the three widgets read as one
/// system: spacing in 4/8/12/16/24, a single hairline in a low-contrast tone,
/// one accent, and no shadows or gradients. Hierarchy comes from type size and
/// weight plus empty space, not from borders and fills.
pub const BASE_CSS: &str = r#"
window {
  background-color: transparent;
  font-family: "undefined";
}

/* Muted text: timestamps, hints, secondary labels. */
.dim {
  color: alpha(@palette_fg, 0.55);
  font-size: 11px;
}

/* Any raised area over the desktop: the sidebar panel, the launcher. A flat
   floating card — no border and no rounded corners, so the three widgets read as
   one hard-edged system rather than a family of pills. */
.surface {
  background-color: alpha(@palette_bg, 0.96);
  color: @palette_fg;
  border-radius: 0;
}

/* Ghost button, shared by the statusbar chips and the sidebar actions: a
   low-contrast fill that only deepens on hover. No border — a hairline here
   would fight the modules for pixels when they sit next to the bar. */
button {
  border: none;
  background-color: alpha(@palette_fg, 0.08);
  background-image: none;
  box-shadow: none;
  color: @palette_fg;
  font-size: 11px;
  font-weight: 500;
  padding: 2px 8px;
  min-height: 20px;
  min-width: 20px;
  /* The default theme rings a focused widget with a 2px accent-coloured
     outline. Nothing here wants that chrome; focus is shown in the palette. */
  outline: 0 solid transparent;
}

button:hover {
  background-color: alpha(@palette_fg, 0.16);
}

button:focus-visible {
  background-color: alpha(@palette_color4, 0.22);
  color: @palette_color4;
}

button:disabled {
  background-color: transparent;
  color: alpha(@palette_fg, 0.35);
}
"#;

thread_local! {
    /// The palette file monitor, kept alive for the life of the process.
    ///
    /// `FileMonitor` stops watching when it is dropped, and nothing else holds
    /// a reference to it: the widgets live for a whole session, so the slot
    /// lives as long as the process does.
    static PALETTE_MONITOR: RefCell<Option<gtk4::gio::FileMonitor>> = const { RefCell::new(None) };
    /// The user-editable widget stylesheet monitor (`~/.config/widgets/style.css`).
    /// When that file changes the widget restyles itself in place, the same as
    /// a palette change does.
    static STYLE_MONITOR: RefCell<Option<gtk4::gio::FileMonitor>> = const { RefCell::new(None) };
}

/// Style a widget: palette, then the shared base, then its own stylesheet.
///
/// The three layers go into a single provider, so the widget sheet always wins
/// over the base and both win over nothing else. The provider is then tied to
/// `~/.config/widgets/palette.css`: when the file changes, the same document is
/// rebuilt from the new palette and loaded back into the same provider, which
/// restyles every widget on the display without a restart.
///
/// If `~/.config/widgets/<name>.css` exists it is used as the widget sheet
/// instead of the compiled-in default, and it is watched for hot-reload — edit
/// the file and the widget restyles itself in place.
pub fn init(widget_css: &'static str, name: &str) {
    let sheet = user_sheet(name).unwrap_or_else(|| widget_css.to_string());
    let provider = palette::provider(&document(&sheet));
    watch(provider, &sheet, name);
}

/// The full stylesheet: the palette first, then the base, then the widget's.
fn document(widget_css: &str) -> String {
    format!("{}\n{}\n{}", palette::palette_css(), BASE_CSS, widget_css)
}

/// The user-editable widget sheet, if they have placed one at
/// `$HOME/.config/widgets/<name>.css`. When present it replaces the compiled-in
/// default so the user's file is what the widget renders.
fn user_sheet(name: &str) -> Option<String> {
    let mut path = palette::style_file()?;
    path.push(format!("{name}.css"));
    std::fs::read_to_string(&path).ok()
}

/// Reload the whole stylesheet on every change to the palette or the user
/// stylesheet (`~/.config/widgets/palette.css` or `~/.config/widgets/style.css`).
///
/// The monitors end up in the thread-locals above, which is what keeps them
/// alive: a `FileMonitor` stops watching the moment it is dropped, and nothing
/// else holds a reference to these.
fn watch(provider: gtk4::CssProvider, widget_css: &str, name: &str) {
    let doc = document(widget_css);
    if let Some(monitor) = watch_palette(&provider, &doc) {
        PALETTE_MONITOR.with(|slot| *slot.borrow_mut() = Some(monitor));
    }
    if let Some(monitor) = watch_user_sheet(&provider, &doc, name) {
        STYLE_MONITOR.with(|slot| *slot.borrow_mut() = Some(monitor));
    }
}

/// Watch the palette. Reloading the same provider with the rebuilt document is
/// what restyles every widget on the display, with no restart.
fn watch_palette(provider: &gtk4::CssProvider, doc: &str) -> Option<gtk4::gio::FileMonitor> {
    let monitor = monitor_file(palette::palette_file()?)?;
    reload_on_change(&monitor, provider, doc);
    Some(monitor)
}

/// Watch the user's own sheet at `$HOME/.config/widgets/<name>.css`, when they
/// have placed one. A missing file is not an error: the widget keeps its shipped
/// look, and nothing is watched until the file appears.
fn watch_user_sheet(
    provider: &gtk4::CssProvider,
    doc: &str,
    name: &str,
) -> Option<gtk4::gio::FileMonitor> {
    let mut path = palette::style_file()?;
    path.push(format!("{name}.css"));
    if std::fs::metadata(&path).is_err() {
        return None;
    }
    let monitor = monitor_file(path)?;
    reload_on_change(&monitor, provider, doc);
    Some(monitor)
}

fn monitor_file(path: std::path::PathBuf) -> Option<gtk4::gio::FileMonitor> {
    gtk4::gio::File::for_path(path)
        .monitor_file(
            gtk4::gio::FileMonitorFlags::NONE,
            gtk4::gio::Cancellable::NONE,
        )
        .ok()
}

/// Load `doc` back into `provider` on every change the monitor reports.
fn reload_on_change(monitor: &gtk4::gio::FileMonitor, provider: &gtk4::CssProvider, doc: &str) {
    let provider = provider.clone();
    let doc = doc.to_string();
    monitor.connect_changed(move |_, _, _, _| provider.load_from_data(&doc));
}

#[cfg(test)]
mod tests;
