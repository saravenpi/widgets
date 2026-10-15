//! The compiled-in artwork of every icon, rasterized from the SVG bodies beside
//! this module.
//!
//! Each mask is one icon's 24×24 pixels in reading order: 24 rows of six hex
//! digits, top to bottom, with the leftmost pixel of a row in the top bit of the
//! first digit. `1` is ink and `0` is transparent, so what the pill paints is
//! this artwork, nibble by nibble, and nothing is decoded at run time.
//!
//! The bodies in `icons/*.svg` are the source these were rasterized from — the
//! [pixelarticons](https://icon-sets.iconify.design/pixelarticons/) set by Gerrit
//! Halfmann (MIT). They are kept so the masks can be rederived when the artwork
//! changes, but nothing reads them at run time: rasterizing SVG needs librsvg,
//! which is a separate package on most distributions, and a machine without it
//! drew a pill's label and nothing beside it.

/// The battery mask: the body capped at both ends, with the terminal on the right.
pub(super) const BATTERY: &str = "\
000000
000000
000000
000000
000000
0ffff0
0ffff0
300030
300030
30003c
30003c
30003c
30003c
30003c
30003c
300030
300030
0ffff0
0ffff0
000000
000000
000000
000000
000000
";

/// The brightness mask: the sun, with a ray above, below and to both sides.
pub(super) const BRIGHTNESS: &str = "\
000000
000000
001800
001800
001800
060060
060060
007e00
007e00
018180
018180
39819c
39819c
018180
018180
007e00
007e00
060060
060060
001800
001800
001800
000000
000000
";

/// The volume mask: the horn on the left, the two waves on the right.
pub(super) const VOLUME: &str = "\
000000
000000
000180
000180
000780
000780
001980
001980
01e180
01e180
018180
018180
018180
018180
01e180
01e180
001980
001980
000780
000780
000180
000180
000000
000000
";

/// The network mask: the signal arcs above, the dot below.
pub(super) const NETWORK: &str = "\
000000
000000
000000
000000
01ff80
01ff80
060060
060060
180018
18ff18
60ff06
6300c6
0300c0
0c0030
0c7e30
007e00
018180
018180
000000
001800
001800
000000
000000
000000
";

/// The logout mask: the power ring, open at the top, with the stem through it.
pub(super) const LOGOUT: &str = "\
000000
000000
001800
001800
0318c0
0318c0
0c1830
0c1830
30180c
30180c
30180c
30000c
30000c
30000c
30000c
30000c
30000c
30000c
0c0030
0c0030
03ffc0
03ffc0
000000
000000
";
