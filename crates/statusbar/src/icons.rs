//! Pixel-art icons for the right-hand control pills.
//!
//! The bodies are the [pixelarticons](https://icon-sets.iconify.design/pixelarticons/)
//! set by Gerrit Halfmann (MIT). Each one is rasterized through librsvg — the
//! SVG loader that ships with gdk-pixbuf, and so with GTK — into a 24×24
//! `Texture`, which is why the bar needs no icon dependency of its own.
//!
//! The tint is the palette's `palette_fg`, substituted into the body's
//! `fill="currentColor"` before rasterizing. A texture is a fixed bitmap, so it
//! cannot follow the stylesheet cascade the way the label beside it does. The
//! icons are drawn once and drawn again by [`follow_palette`] whenever the
//! stylesheet is rebuilt, so a `palette sync` moves the glyphs and the text
//! together instead of leaving the icons a release behind.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk::RGBA;
use gtk4::gdk_pixbuf;
use gtk4::gio;
use gtk4::glib;

/// The icons the bar draws. Only the ones it actually uses are compiled in.
#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Battery,
    Brightness,
    Volume,
    Network,
    Logout,
}

impl Icon {
    /// The icon's SVG document, untinted. `fill="currentColor"` is what
    /// [`svg_with_tint`] replaces.
    fn svg(self) -> &'static str {
        match self {
            Self::Battery => include_str!("icons/battery.svg"),
            Self::Brightness => include_str!("icons/sun.svg"),
            Self::Volume => include_str!("icons/volume.svg"),
            Self::Network => include_str!("icons/wifi.svg"),
            Self::Logout => include_str!("icons/power.svg"),
        }
    }
}

/// The icons a widget has painted, remembered so a palette change can repaint
/// them.
///
/// A pill asks for its image once, when the bar is built; the images it gets
/// back stay in here, which is what lets [`retint`](Painted::retint) find them
/// again after the palette has moved.
#[derive(Default)]
pub struct Painted(Vec<(Icon, gtk4::Image)>);

impl Painted {
    /// Paint `icon` in the palette's current foreground, and remember it. `None`
    /// when the icon cannot be rasterized at all, so the caller can fall back to
    /// a label-only pill.
    pub fn image(&mut self, icon: Icon) -> Option<gtk4::Image> {
        let image = gtk4::Image::from_paintable(Some(&texture(icon, &tint())?));
        self.0.push((icon, image.clone()));
        Some(image)
    }

    /// Draw every icon again in the palette's current foreground.
    pub fn retint(&self) {
        let tint = tint();
        for (icon, image) in &self.0 {
            if let Some(texture) = texture(*icon, &tint) {
                image.set_paintable(Some(&texture));
            }
        }
    }
}

/// Repaint `icons` every time the stylesheet is rebuilt.
///
/// The labels follow the cascade on their own; a texture cannot, so this is what
/// brings the glyphs along when `palette sync` rewrites `palette.css` — and what
/// keeps them from being a restart behind the bar they sit on.
pub fn follow_palette(icons: Rc<RefCell<Painted>>) {
    shell_common::theme::on_reload(move || icons.borrow().retint());
}

/// The ink the glyphs are drawn in: the palette's foreground, or a neutral grey
/// when it cannot be parsed, so a pill is never left without its icon.
fn tint() -> RGBA {
    shell_common::palette::palette_fg_rgba().unwrap_or_else(|| RGBA::new(0.5, 0.5, 0.5, 1.0))
}

/// The tinted 24×24 `Texture` for one icon.
pub fn texture(icon: Icon, tint: &RGBA) -> Option<gtk4::gdk::Texture> {
    let doc = svg_with_tint(icon.svg(), tint);
    let pixbuf = rasterize(doc.as_bytes())?;
    Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf))
}

/// Replace the body's `currentColor` with a concrete fill.
///
/// `currentColor` resolves against the `color` property, whose support varies
/// with the librsvg version. Substituting the hex up front keeps the result the
/// same everywhere.
fn svg_with_tint(svg: &str, tint: &RGBA) -> String {
    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        (tint.red() * 255.0).round() as u8,
        (tint.green() * 255.0).round() as u8,
        (tint.blue() * 255.0).round() as u8,
    );
    svg.replace("currentColor", &hex)
}

/// Rasterize SVG bytes through the gdk-pixbuf SVG loader (librsvg).
///
/// The stream owns its own copy of the bytes, so the caller's buffer can go as
/// soon as this returns, and the pixbuf owns its pixels, so it can be dropped as
/// soon as the texture has taken its reference.
fn rasterize(svg: &[u8]) -> Option<gdk_pixbuf::Pixbuf> {
    let bytes = glib::Bytes::from(svg);
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    gdk_pixbuf::Pixbuf::from_stream(&stream, gio::Cancellable::NONE).ok()
}

#[cfg(test)]
mod tests;
