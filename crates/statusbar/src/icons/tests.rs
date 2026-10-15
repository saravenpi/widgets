//! The icon checks: every icon has a mask of its own, the tint reaches the pixels
//! the mask inks, and a rewritten palette repaints the icons the bar is already
//! showing.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::{gdk_pixbuf, gio};

use super::*;
use crate::testing;

/// Every icon has to decode to a full 24×24 frame with ink on it, and the five
/// have to be five different pictures.
///
/// A mask that is a row short, empty, or copied from its neighbour paints a blank
/// or wrong pill, and nothing else in the suite would notice: the icons are the
/// one part of the bar no stylesheet check touches.
#[test]
fn every_icon_decodes_to_a_full_frame_of_its_own() {
    let mut seen: Vec<&str> = Vec::new();
    for icon in ALL {
        let pixels = rgba(icon, &RGBA::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(
            pixels.len(),
            (SIDE * SIDE * 4) as usize,
            "{icon:?} is not a 24×24 frame"
        );
        let inked = pixels.chunks(4).filter(|pixel| pixel[3] == 255).count();
        assert!(inked > 0, "{icon:?} paints nothing");
        assert!(
            inked < (SIDE * SIDE) as usize,
            "{icon:?} paints a solid block"
        );
        assert!(
            !seen.contains(&icon.mask()),
            "{icon:?} repeats another icon's mask"
        );
        seen.push(icon.mask());
    }
}

/// The tint has to reach the pixels the mask inks: if it did not, the icons would
/// paint in whatever the bytes happened to hold, which is nothing on a palette
/// the bar was not built for.
#[test]
fn the_tint_reaches_the_inked_pixels() {
    let pixels = rgba(Icon::Battery, &RGBA::new(1.0, 0.0, 0.0, 1.0));
    let ink: &[u8] = &[0xff, 0x00, 0x00, 0xff];
    let blank: &[u8] = &[0x00, 0x00, 0x00, 0x00];
    let mut inked = 0;
    for pixel in pixels.chunks(4) {
        assert!(
            pixel == ink || pixel == blank,
            "a pixel is neither the tint nor transparent: {pixel:?}"
        );
        if pixel == ink {
            inked += 1;
        }
    }
    assert!(inked > 0, "the tint never landed on a pixel");
}

/// `palette sync` rewrites `palette.css`, the bar restyles itself, and the glyphs
/// have to come along. They are bitmaps rather than rules, so the only way they
/// take the new ink is to be drawn again — which is the job `follow_palette`
/// exists for. The rewrite is done twice to show the icons stay attached, not
/// just move the once.
///
/// This is real GTK on a real display, so it stands down where there is none
/// rather than fail on a machine that cannot draw an icon.
#[test]
fn a_palette_rewrite_retints_the_icons() {
    let ran = testing::on_gtk_thread(|| {
        // A main context the test drives itself, and the one the palette monitor
        // is created in below. GTK takes the default context for its own when it
        // starts on this thread, and a monitor attached to that would only be
        // driven by GTK's loop — which a test never runs.
        let context = glib::MainContext::new();
        let _ = context.with_thread_default(|| retint_check(&context));
    });
    if !ran {
        eprintln!("statusbar: no display, skipping the icon retint test");
    }
}

/// The check itself: boot the stylesheet the bar uses, paint an icon, rewrite
/// `palette.css` the way `palette sync` does, and watch the glyph follow it —
/// twice, so a rewrite that moved the icons the once would not pass.
fn retint_check(context: &glib::MainContext) {
    let home = std::env::temp_dir().join("widgets-icon-retint-test");
    // Start from nothing: a palette or sheet a failed run left behind would answer here.
    let _ = std::fs::remove_dir_all(&home);
    let previous = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    palette_at(&home, "#ff0000");

    shell_common::theme::init(include_str!("../../style.css"), "statusbar");
    let painted = Rc::new(RefCell::new(Painted::default()));
    follow_palette(painted.clone());
    let image = painted.borrow_mut().image(Icon::Battery);
    assert_eq!(
        glyph_colour(&image),
        Some((0xff, 0x00, 0x00)),
        "the glyph takes the ink the palette names"
    );

    let inks = [
        ("#00ff00", (0x00, 0xff, 0x00)),
        ("#0000ff", (0x00, 0x00, 0xff)),
    ];
    for (palette, expected) in inks {
        palette_at(&home, palette);
        assert!(
            wait_until(context, || glyph_colour(&image) == Some(expected)),
            "a palette sync has to repaint the glyphs"
        );
    }

    match previous {
        Some(home) => std::env::set_var("HOME", home),
        None => std::env::remove_var("HOME"),
    }
    let _ = std::fs::remove_dir_all(&home);
}

/// The colour of the first fully opaque pixel of an image's texture. The body is
/// one flat fill, so that pixel is the glyph's own ink; the edges are
/// anti-aliased and only a solid pixel answers.
fn glyph_colour(image: &gtk4::Image) -> Option<(u8, u8, u8)> {
    let texture = image.paintable()?.downcast::<gtk4::gdk::Texture>().ok()?;
    let bytes = texture.save_to_png_bytes();
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    let pixbuf = gdk_pixbuf::Pixbuf::from_stream(&stream, gio::Cancellable::NONE).ok()?;
    let channels = pixbuf.n_channels() as usize;
    let pixels = pixbuf.read_pixel_bytes();
    pixels
        .chunks(channels)
        .find(|pixel| pixel[channels - 1] == 255)
        .map(|pixel| (pixel[0], pixel[1], pixel[2]))
}

/// Write a palette into `dir` naming `fg` as the foreground, the way `palette
/// sync` writes it, so the widget under test reads it back from disk. Every other
/// token is filled in too, so the document the bar loads is complete.
fn palette_at(dir: &std::path::Path, fg: &str) {
    let mut sheet = format!("@define-color palette_fg {fg};\n");
    let others = [
        "bg", "cursor", "color0", "color1", "color2", "color3", "color4", "color5", "color6",
        "color7", "color8", "color9", "color10", "color11", "color12", "color13", "color14",
        "color15",
    ];
    for name in others {
        sheet.push_str(&format!("@define-color palette_{name} #19120c;\n"));
    }

    let config = dir.join(".config/widgets");
    std::fs::create_dir_all(&config).expect("the test config directory");
    std::fs::write(config.join("palette.css"), sheet).expect("write the palette");
}

/// Pump `context` until `check` holds or the deadline passes. The palette monitor
/// reports into the context it was created in and no window is on screen to drive
/// it, so the test drives that one itself.
fn wait_until(context: &glib::MainContext, check: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        while context.iteration(false) {}
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    check()
}
