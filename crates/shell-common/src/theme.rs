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
/// A missing file is not an error: the palette layer falls back to the built-in
/// colours, and the user stylesheet layer is simply dropped when the file is
/// absent (the widget keeps its shipped look). When the file appears later the
/// monitor reports the creation and the widget picks it up.
fn watch(provider: gtk4::CssProvider, widget_css: &str, name: &str) {
    let doc = document(widget_css);
    let palette_path = match palette::palette_file() {
        Some(p) => p,
        None => return,
    };
    let palette_monitor = gtk4::gio::File::for_path(palette_path)
        .monitor_file(
            gtk4::gio::FileMonitorFlags::NONE,
            gtk4::gio::Cancellable::NONE,
        );
    let Ok(palette_monitor) = palette_monitor else {
        return;
    };
    let doc_for_palette = doc.clone();
    let palette_provider = provider.clone();
    palette_monitor.connect_changed(move |_, _, _, _| {
        palette_provider.load_from_data(&doc_for_palette);
    });
    PALETTE_MONITOR.with(|slot| *slot.borrow_mut() = Some(palette_monitor));

    // Watch the user-editable widget stylesheet, if it exists.
    let style_path = match palette::style_file() {
        Some(dir) => {
            let mut p = dir;
            p.push(format!("{name}.css"));
            p
        }
        None => return,
    };
    if std::fs::metadata(&style_path).is_ok() {
        let style_monitor = gtk4::gio::File::for_path(style_path)
            .monitor_file(
                gtk4::gio::FileMonitorFlags::NONE,
                gtk4::gio::Cancellable::NONE,
            );
        let Ok(style_monitor) = style_monitor else {
            return;
        };
        let doc_for_style = doc.clone();
        let style_provider = provider.clone();
        style_monitor.connect_changed(move |_, _, _, _| {
            style_provider.load_from_data(&doc_for_style);
        });
        STYLE_MONITOR.with(|slot| *slot.borrow_mut() = Some(style_monitor));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    /// What GTK refused to parse. A stylesheet typo never fails a widget at
    /// runtime - the rule simply stops applying - so the tests collect these.
    ///
    /// The provider is built through `glib::Object::new` on purpose. The
    /// generated `CssProvider::new` asserts that `gtk::init` has run, and that
    /// needs a display, while parsing does not. Staying display-free is what
    /// lets the check run in CI, where no compositor exists.
    fn parse_errors(document: &str) -> Vec<String> {
        let errors = Rc::new(RefCell::new(Vec::new()));
        let sink = errors.clone();
        let provider: gtk4::CssProvider = gtk4::glib::Object::new();
        provider.connect_parsing_error(move |_, _, error| {
            sink.borrow_mut().push(error.to_string());
        });
        provider.load_from_data(document);
        let collected = errors.borrow().clone();
        collected
    }

    fn widget_sheet(widget: &str) -> String {
        let path = format!(
            "{}/../{}/style.css",
            env!("CARGO_MANIFEST_DIR"),
            widget
        );
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
    }

    /// Every stylesheet the widgets ship has to parse, on any machine: a rule
    /// GTK rejects stops applying silently, so nothing else would notice.
    ///
    /// Every GTK call stays in this one test on purpose. GTK is not thread
    /// safe, and two tests parsing at the same time report errors that are not
    /// there - "No property named border" for a border that parses fine on its
    /// own - so the work has to stay on a single thread.
    #[test]
    fn stylesheets_parse() {
        let palette_and_base = format!("{}\n{}", palette::palette_css(), BASE_CSS);
        assert_eq!(
            parse_errors(&palette_and_base),
            Vec::<String>::new(),
            "the palette and the shared base sheet must parse"
        );

        for widget in ["statusbar", "sidebar", "launcher"] {
            let document = format!("{}\n{}", palette_and_base, widget_sheet(widget));
            assert_eq!(
                parse_errors(&document),
                Vec::<String>::new(),
                "{widget} style.css does not parse on top of the shared base"
            );
        }

        // The assertions above only prove something while GTK reports what it
        // refuses. Without these, a change that stopped collecting errors would
        // leave the check green and empty instead of failing.
        assert!(!parse_errors("window { colour: red; }").is_empty());
        assert!(!parse_errors("window { background-color: notacolor; }").is_empty());
        assert!(parse_errors("window { background-color: #19120c; }").is_empty());
    }

    /// The reload path rebuilds the whole document; the widget layer has to be
    /// in there, or a palette change would quietly drop a widget's own rules.
    #[test]
    fn document_layers_are_all_present() {
        let widget = ".statusbar { color: @palette_fg; }";
        let full = document(widget);
        assert!(full.contains(BASE_CSS), "the shared base is missing");
        assert!(full.ends_with(widget), "the widget sheet is missing");
        assert!(
            full.contains("palette_bg"),
            "the palette layer is missing"
        );
    }
}
