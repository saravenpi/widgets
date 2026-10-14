//! Pixel-art icons for the right-hand control pills.
//!
//! The bodies are the [pixelarticons](https://icon-sets.iconify.design/pixelarticons/)
//! set by Gerrit Halfmann (MIT). Each one is rasterized through librsvg — the
//! SVG loader that ships with gdk-pixbuf, and so with GTK — into a 24×24
//! `Texture`, which is why the bar needs no icon dependency of its own.
//!
//! The tint is the palette's `palette_fg`, substituted into the body's
//! `fill="currentColor"` before rasterizing. A texture is a fixed bitmap, so it
//! cannot follow the stylesheet cascade: the icons are read once when the bar is
//! built, and a later `palette sync` restyles the text but not the glyphs. A
//! restart is what retints them.

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

/// A 24×24 `Image` painting the icon in the palette foreground — the same ink as
/// the label beside it. A neutral grey stands in when the palette cannot be
/// parsed, so a pill is never left without its icon. `None` when the icon cannot
/// be rasterized at all, so the caller can fall back to a label-only pill.
pub fn image(icon: Icon) -> Option<gtk4::Image> {
    let tint = shell_common::palette::palette_fg_rgba().unwrap_or_else(|| RGBA::new(0.5, 0.5, 0.5, 1.0));
    let texture = texture(icon, &tint)?;
    Some(gtk4::Image::from_paintable(Some(&texture)))
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
mod tests {
    use super::*;

    /// Every icon the bar uses has to rasterize to a 24×24 bitmap with ink on
    /// it. Without this, a missing SVG loader or a body that no longer paints
    /// would show up as a blank pill and nothing else would notice — the icons
    /// are the one part of the bar no stylesheet check touches.
    #[test]
    fn every_icon_rasterizes_with_visible_pixels() {
        let icons = [
            Icon::Battery,
            Icon::Brightness,
            Icon::Volume,
            Icon::Network,
            Icon::Logout,
        ];
        for icon in icons {
            let doc = svg_with_tint(icon.svg(), &RGBA::new(1.0, 1.0, 1.0, 1.0));
            let pixbuf =
                rasterize(doc.as_bytes()).unwrap_or_else(|| panic!("{icon:?} did not rasterize"));
            assert_eq!(
                (pixbuf.width(), pixbuf.height()),
                (24, 24),
                "{icon:?} is not 24×24"
            );
            assert!(pixbuf.has_alpha(), "{icon:?} lost its alpha channel");
            let channels = pixbuf.n_channels() as usize;
            let pixels = pixbuf.read_pixel_bytes();
            assert!(
                pixels.chunks(channels).any(|pixel| pixel[channels - 1] > 0),
                "{icon:?} rasterized to an empty bitmap"
            );
        }
    }

    /// The tint has to reach the glyph: if it did not, the icons would render
    /// white-on-transparent and be invisible on a light palette.
    #[test]
    fn the_tint_replaces_the_bodies_colour() {
        let doc = svg_with_tint(Icon::Battery.svg(), &RGBA::new(1.0, 0.0, 0.0, 1.0));
        assert!(doc.contains("#ff0000"), "the tint was not substituted in");
        assert!(!doc.contains("currentColor"), "currentColor was left behind");
    }

    /// A body that cannot be rasterized has to come back as `None` rather than
    /// panicking, because that is what lets the pill fall back to its label.
    #[test]
    fn unparseable_svg_is_none() {
        assert!(rasterize(b"<svg>not svg at all").is_none());
    }

}
