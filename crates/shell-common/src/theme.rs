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
//! restart. The reload rereads what it is built from — see [`rebuild`] — because
//! a document captured at start-up would be loaded back unchanged, which is the
//! same as not watching at all.

use std::cell::RefCell;

mod watch;

use crate::palette;

/// The gap around a floating widget and the window rectangles it sits inside,
/// matching `gaps outer` in the compositor config. The bar is inset by it on
/// every side, and the sidebar and its toasts keep it off the right and bottom
/// edges so they float the same way.
///
/// This is the *same unit* sway uses for `gaps`. A sway gap and a layer-shell
/// margin are both logical (device-independent) pixels — sway multiplies its
/// config values by the output scale to place things physically, and GTK does
/// the same with `set_margin` (surface-local coordinates) and CSS `px`. So
/// `gaps outer 10` and `GAP = 10` are the same 10 pixels on screen at any
/// scale, and the numbers can be kept literally equal. Read the value from
/// `gaps outer` in the compositor config; it is what the windows respect too.
pub const GAP: i32 = 10;
/// `gaps inner` in the compositor config, the same measure used between the bar
/// and the windows it sits beside.
pub const GAP_INNER: i32 = 10;

/// The gap between two modules on a widget and between the items inside one.
///
/// One measure for both, from the 4/8/12/16/24 scale the shared sheet follows,
/// so a row reads as a single rhythm: the statusbar's workspace chips, its CPU
/// and memory pills and its control pills are all this far apart, and one module
/// is this far from the next. It used to be 6 inside a module and 0 between two
/// of them, which left the CPU and memory pills welded to the control row while
/// every other pair on the bar had a gap.
pub const MODULE_GAP: i32 = 8;

/// Height of the statusbar, and the strip it reserves at the top of the
/// screen. A widget that docks to an edge keeps its content out of that strip
/// so it never sits underneath the bar. The bar itself is a floating pill, so
/// this is the pill (32px) plus a `GAP` above and below it.
///
/// The bar's window is exactly this tall, so this is not an estimate the
/// sidebar has to guess around: `.bar` fills 32px of content between two `GAP`
/// margins and no vertical padding, which sums to this. `statusbar` reserves it
/// as its exclusive zone, so windows and the sidebar both start below it.
pub const BAR_HEIGHT: i32 = 32 + GAP;
pub const WINDOW_TOP_GAP: i32 = BAR_HEIGHT + GAP;

/// Loaded before every widget stylesheet, which may override any of it.
///
/// The rules here follow one strict scale so the three widgets read as one
/// system: spacing in 4/8/12/16/24, a single hairline in a low-contrast tone,
/// one accent, and no shadows or gradients. Hierarchy comes from type size and
/// weight plus empty space, not from borders and fills.
pub const BASE_CSS: &str = include_str!("theme/base.css");

thread_local! {
    /// The callbacks to run after the stylesheet is rebuilt, in the order they
    /// were registered. See [`on_reload`].
    static RELOADS: RefCell<Vec<Box<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

/// Style a widget: palette, then the shared base, then its own stylesheet.
///
/// The three layers go into a single provider, so the widget sheet always wins
/// over the base and both win over nothing else. The provider is then tied to
/// `~/.config/widgets/palette.css`: when the file changes, the document is
/// rebuilt from the new palette and loaded back into the same provider, which
/// restyles every widget on the display without a restart.
///
/// If `~/.config/widgets/<name>.css` exists it is used as the widget sheet
/// instead of the compiled-in default, and it is watched for hot-reload — edit
/// the file and the widget restyles itself in place.
pub fn init(widget_css: &'static str, name: &str) {
    let provider = palette::provider(&rebuild(name, widget_css));
    watch::watch(provider, name, widget_css);
}

/// Run `callback` after every stylesheet rebuild: a `palette sync`, or an edit
/// to the user's own sheet.
///
/// Most of what a widget draws follows the cascade by itself and needs nothing
/// here. This is for the parts that cannot: a bitmap is a fixed texture rather
/// than a rule, so the only way it takes the new palette is to be drawn again.
/// The statusbar's icons are the case that made this necessary.
///
/// A callback must not register another one — the list is borrowed while they
/// run.
pub fn on_reload(callback: impl Fn() + 'static) {
    RELOADS.with(|slot| slot.borrow_mut().push(Box::new(callback)));
}

/// The stylesheet a widget draws itself from, read from disk now: the palette,
/// the shared base, and the widget's own sheet.
///
/// Read rather than remembered. Both `init` and the reload go through here, so
/// what is loaded on a change is the files as they are, not a copy of what they
/// said when the widget started.
pub(crate) fn rebuild(name: &str, default: &str) -> String {
    document(&user_sheet(name).unwrap_or_else(|| default.to_string()))
}

/// The full stylesheet: the palette first, then the base, then the widget's.
fn document(widget_css: &str) -> String {
    let opacity = crate::config::get().general.opacity;
    if (0.0..1.0).contains(&opacity) {
        let targets = ".surface, .bar, .settings-dialog, .exit-dialog, \
                       .popup, .panel, .toast, .cheatsheet-dialog";
        format!(
            "{}\n{}\n{}\n{targets} {{ background-color: alpha(@palette_bg, {:.2}); }}",
            palette::palette_css(),
            BASE_CSS,
            widget_css,
            opacity
        )
    } else {
        format!("{}\n{}\n{}", palette::palette_css(), BASE_CSS, widget_css)
    }
}

/// The sheet the user placed at `$HOME/.config/widgets/<name>.css`. When present
/// it replaces the compiled-in default so the user's file is what the widget
/// renders.
fn user_sheet(name: &str) -> Option<String> {
    std::fs::read_to_string(palette::sheet_file(name)?).ok()
}

/// Tell every listener the stylesheet moved under them.
fn notify_reload() {
    RELOADS.with(|slot| {
        for callback in slot.borrow().iter() {
            callback();
        }
    });
}

#[cfg(test)]
mod tests;
