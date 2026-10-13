//! The shared visual language of every widget.
//!
//! One base stylesheet defines what all three widgets agree on — transparent
//! windows, muted text, ghost buttons, panel surfaces — so a widget's own
//! stylesheet only has to say what makes it different. Colours come from the
//! palette file (`~/.config/widgets/palette.css`), never from literals.

use crate::palette;

/// Height of the statusbar, and the strip it reserves at the top of the
/// screen. A widget that docks to an edge keeps its content out of that strip
/// so it never sits underneath the bar.
pub const BAR_HEIGHT: i32 = 32;

/// Loaded before every widget stylesheet, which may override any of it.
///
/// The rules here follow one strict scale so the three widgets read as one
/// system: spacing in 4/8/12/16/24, a single hairline in a low-contrast tone,
/// one accent, and no shadows or gradients. Hierarchy comes from type size and
/// weight plus empty space, not from borders and fills.
pub const BASE_CSS: &str = r#"
window {
  background-color: transparent;
}

/* Muted text: timestamps, hints, the compositor badge. */
.dim {
  color: alpha(@palette_fg, 0.55);
  font-size: 11px;
}

/* Any raised area over the desktop: the sidebar panel, the launcher. */
.surface {
  background-color: @palette_bg;
  color: @palette_fg;
  border: 1px solid alpha(@palette_fg, 0.14);
  border-radius: 4px;
}

/* Ghost button, shared by the statusbar chips and the sidebar actions: a
   hairline box that only fills in on hover. */
button {
  border: 1px solid alpha(@palette_fg, 0.14);
  background-color: transparent;
  background-image: none;
  box-shadow: none;
  color: @palette_fg;
  font-size: 11px;
  font-weight: 500;
  padding: 2px 8px;
  min-height: 20px;
  min-width: 20px;
  border-radius: 4px;
  /* The default theme rings a focused widget with a 2px accent-coloured
     outline. Nothing here wants that chrome; focus is shown in the palette. */
  outline: 0 solid transparent;
}

button:hover {
  background-color: alpha(@palette_fg, 0.08);
  border-color: alpha(@palette_fg, 0.3);
}

button:focus-visible {
  border-color: alpha(@palette_color4, 0.7);
  color: @palette_color4;
}

button:disabled {
  background-color: transparent;
  border-color: alpha(@palette_fg, 0.06);
  color: alpha(@palette_fg, 0.35);
}
"#;

/// Style a widget: palette, then the shared base, then its own stylesheet.
///
/// The three layers go into a single provider, so the widget sheet always wins
/// over the base and both win over nothing else.
pub fn init(widget_css: &str) {
    let document = format!(
        "{}\n{}\n{}",
        palette::palette_css(),
        BASE_CSS,
        widget_css
    );
    palette::load(&document);
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
}
