//! Pixel-art icons for the right-hand control pills.
//!
//! The bodies are the [pixelarticons](https://icon-sets.iconify.design/pixelarticons/)
//! set by Gerrit Halfmann (MIT), compiled in as the 24×24 masks in [`masks`] and
//! painted from those into a texture. Nothing is rasterized at run time: the SVG
//! loader (librsvg) is a separate package on most distributions, and a machine
//! without it drew a pill's label and nothing beside it, silently.
//!
//! The tint is the palette's `palette_fg`. A texture is a fixed bitmap and cannot
//! follow the stylesheet cascade the way the label next to it does, so the icons
//! are painted once and painted again by [`follow_palette`] whenever the
//! stylesheet is rebuilt — which is what moves the glyphs and the text together
//! after a `palette sync`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk::{MemoryFormat, MemoryTexture, Texture, RGBA};
use gtk4::glib;
use gtk4::prelude::*;

mod masks;

/// The side of every icon, in pixels.
const SIDE: i32 = 24;

/// The icons the bar draws. Only the ones it actually uses are compiled in.
#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Battery,
    Brightness,
    Volume,
    Network,
    Logout,
}

/// Every icon, in the order the control row reads them. One list, so a new icon
/// is added in one place and the checks and the diagnostic report both follow.
pub const ALL: [Icon; 5] = [
    Icon::Battery,
    Icon::Brightness,
    Icon::Volume,
    Icon::Network,
    Icon::Logout,
];

impl Icon {
    /// The icon's 24×24 mask, the artwork it paints from.
    fn mask(self) -> &'static str {
        match self {
            Self::Battery => masks::BATTERY,
            Self::Brightness => masks::BRIGHTNESS,
            Self::Volume => masks::VOLUME,
            Self::Network => masks::NETWORK,
            Self::Logout => masks::LOGOUT,
        }
    }
}

/// The icons a widget has painted, remembered so a palette change can repaint
/// them.
///
/// A pill asks for its image once, when the bar is built; the images it gets back
/// stay in here, which is what lets [`retint`](Painted::retint) find them again
/// after the palette has moved.
#[derive(Default)]
pub struct Painted(Vec<(Icon, gtk4::Image)>);

impl Painted {
    /// Paint `icon` in the palette's current foreground, and remember it.
    pub fn image(&mut self, icon: Icon) -> gtk4::Image {
        let image = gtk4::Image::from_paintable(Some(&texture(icon, &tint())));
        self.0.push((icon, image.clone()));
        image
    }

    /// Paint every icon again in the palette's current foreground.
    pub fn retint(&self) {
        let tint = tint();
        for (icon, image) in &self.0 {
            image.set_paintable(Some(&texture(*icon, &tint)));
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
pub(crate) fn tint() -> RGBA {
    shell_common::palette::palette_fg_rgba().unwrap_or_else(|| RGBA::new(0.5, 0.5, 0.5, 1.0))
}

/// The 24×24 texture one icon paints as.
pub(crate) fn texture(icon: Icon, tint: &RGBA) -> Texture {
    let pixels = glib::Bytes::from(&rgba(icon, tint)[..]);
    let stride = SIDE as usize * 4;
    MemoryTexture::new(
        SIDE,
        SIDE,
        MemoryFormat::R8g8b8a8Premultiplied,
        &pixels,
        stride,
    )
    .upcast()
}

/// The mask's pixels as RGBA bytes, in reading order: `tint` on every pixel the
/// mask inks, fully transparent everywhere else.
///
/// The alpha is either 0 or 255, so premultiplied and straight alpha are the same
/// bytes and the format the texture is built with does not change the result.
fn rgba(icon: Icon, tint: &RGBA) -> Vec<u8> {
    let ink = [
        (tint.red() * 255.0).round() as u8,
        (tint.green() * 255.0).round() as u8,
        (tint.blue() * 255.0).round() as u8,
    ];
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for row in icon.mask().lines() {
        for inked in row.as_bytes().iter().flat_map(|digit| nibble_bits(*digit)) {
            let pixel = match inked {
                true => [ink[0], ink[1], ink[2], 255],
                false => [0, 0, 0, 0],
            };
            pixels.extend_from_slice(&pixel);
        }
    }
    pixels
}

/// The four pixels one hex digit of a mask stands for, leftmost first.
fn nibble_bits(digit: u8) -> [bool; 4] {
    let value = (digit as char).to_digit(16).unwrap_or(0);
    [3, 2, 1, 0].map(|bit| value & (1 << bit) != 0)
}

#[cfg(test)]
mod tests;
