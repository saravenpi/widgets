# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.3] - 2026-10-15

### Fixed

- `statusbar`: the control icons are compiled into the binary as the 24×24 masks
  they rasterize to, so nothing is decoded at run time. They used to be
  rasterized from their SVG bodies through gdk-pixbuf's SVG loader, which is a
  separate package on most distributions (`librsvg` on many of them) and not
  something GTK pulls in on its own: on a machine without it the rasterization
  failed, and a pill that cannot draw its icon simply omits it, so the bar showed
  every label and no glyph and printed nothing to say why.

## [0.9.2] - 2026-10-15

### Fixed

- `statusbar`: the battery pill says when the machine is plugged in. The adapter
  read asked the *battery* directory for `AC/online` — a file that belongs to the
  adapter and that no battery device has — so it resolved to nothing on every
  machine, and the state machine required it before it would call a battery
  charging. A battery on external power therefore showed `Bat 80?` while charging
  and while held at its level. The adapter is found by scanning
  `/sys/class/power_supply` for the supply that reports `online`, whatever the
  machine calls it, a battery that reports `Charging` is taken at its word, and
  the popup no longer claims a time to empty for a battery that is plugged in and
  not draining.
- `statusbar`: the control icons follow the palette. They are drawn as
  rasterized bitmaps rather than stylesheet rules, so `palette sync` used to
  restyle the bar around them and leave the glyphs on the old ink until the bar
  was restarted; the pill now hands its icon to `icons::follow_palette`, which
  paints it again whenever the stylesheet is rebuilt.
- The stylesheet reload builds the document from the files again instead of
  loading back the one captured at start-up. A widget that was already up kept
  the palette it started with: the file was watched, and the change was thrown
  away. The same held for a sheet the user edited by hand.
- `shell-common`: a widget's own sheet is read from
  `~/.config/widgets/<name>.css`. The path appended the widget name to
  `style.css`, which is a file rather than a directory, so it never resolved and
  a sheet the user placed was never picked up.

### Changed

- Tests that build widgets run their body on one toolkit thread instead of
  calling GTK from whichever test thread reached it first, and report the same
  quiet stand-down when there is no display.

## [0.9.1] - 2026-10-15

### Added

- `launcher`: a test for the search surface. It builds the query field and the
  result list for real, so it checks that the query decides what is on screen,
  that the first match comes selected, and that the arrow keys stay inside the
  list. Like the sidebar's panel check it stands down when there is no display.

### Changed

- The code the widgets ship is arranged by responsibility instead of by when it
  was written. No behaviour changes; the suites pass before and after.
  - `statusbar`: the bar itself is `bar.rs`. The battery, the network, the
    system stats and the `sway` workspace state each split their probe from the
    surface they feed, and the sysfs reads `battery` and `brightness` had grown
    private copies of live in one `sysfs.rs`.
  - `sidebar`: `main.rs` is the entry point and the pid file. The panel is
    `panel.rs`, its rows are `panel/list.rs`, its actions are
    `panel/actions.rs`, and the notification daemon is `dbus.rs` with the
    connection it runs on in `dbus/runtime.rs`.
  - `launcher`: `main.rs` is the entry point and the window placement, the
    search surface is `popup.rs`, and the `Exec` splitter is a named state
    machine rather than one long loop.
  - `shell-common`: `theme::watch` is split into the palette half and the
    user-sheet half, and the stylesheet checks moved beside the module.
- Tests that build widgets now live in files beside the modules they check, so
  no module counts its own checks as part of its surface.
- `make check` runs `filet check` after `cargo check`, so the size, nesting and
  complexity limits are part of the repository's own gate rather than a report
  nobody runs.
- `filet.yml` moves `limits.params` from 5 to 9: the freedesktop `Notify`
  signature is eight arguments plus `&self`, and zbus derives the D-Bus
  signature from the Rust parameter list, so that one shim cannot be shorter.
  The reasoning is in the comment there.

## [0.9.0] - 2026-10-15

### Added

- `statusbar`: every control pill now leads with a pixel-art glyph, from the
  [pixelarticons](https://icon-sets.iconify.design/pixelarticons/) set. The SVGs
  are rasterized through librsvg, the SVG loader GTK already links, so the bar
  takes on no icon dependency of its own. They are drawn in the palette
  foreground read from `~/.config/widgets/palette.css`. A texture is a bitmap,
  not a stylesheet rule, so the glyphs take that ink once, when the bar is
  built: a later `palette sync` restyles the bar around them, and restarting the
  bar is what retints them.

### Fixed

- `statusbar`: the logout pill ended the session as its own availability check.
  Deciding whether to draw the button ran `loginctl terminate-session`, so
  starting the bar logged the user out. The check is now a read
  (`loginctl show-session`) and never ends the session it is asking about.
- `statusbar`: clicking a control pill did nothing. The pill wiring dropped the
  popup instead of showing it, so volume, brightness, battery and network all
  lost the popups they are supposed to open.
- `statusbar`: the battery and network tickers refreshed a label that was never
  on the bar. Each pill built its button from a copy of the pill's text, so the
  label the ticker held was orphaned and the visible one never changed. The pill
  now paints the very label the ticker updates.
- `statusbar`: the brightness pill was placed on machines whose backlight is
  read-only, because the writability check was a write that set the backlight to
  `0`. The check opens the attribute for writing without touching its value, and
  a read-only device hides the pill instead of offering a dead control.
- `statusbar`: the popup's value label carried the pill's whole sentence, so a
  brightness step showed `Brightness 42` where the popup reads `42`.

### Changed

- `statusbar`: the control row, the pill that wraps each control, and the fill
  bar and fallback message every popup shares moved out of `main.rs` into
  `control.rs` and `popup.rs`, so `main.rs` wires the bar rather than rebuilding
  it.
- `statusbar`: refreshing a pill costs one probe of its backend instead of one
  per value read — the battery takes a single snapshot per call — replacing the
  short-lived probe caches three of the modules had grown.
- The workspace version now matches the release. `v0.8.0` shipped while
  `Cargo.toml` still said `0.7.0`, so the released binaries reported the wrong
  version; this release carries `0.9.0` in both places.

## [0.8.0] - 2026-10-14

### Added

- `statusbar`: a logout pill that ends the current session through
  `loginctl terminate-session $XDG_SESSION_ID`, the same action a compositor
  logout binding (`mod+shift+e`) should map to. The pill is hidden when the
  session id is unknown or `loginctl` is absent, so the bar never offers a dead
  button.
- `statusbar`: a volume pill driven by PulseAudio / PipeWire through `pactl`.
  Clicking the pill opens a small popup with the current percent, a fill bar, and
  `+1%`/`-1%` steps; the pill updates to the new level after each change.
- `statusbar`: a brightness pill driven by the ACPI backlight class in
  `/sys/class/backlight`. Clicking opens a popup with the percent, a fill bar,
  and `+5%`/`-5%` steps; the pill follows the new level after each change.
- `statusbar`: a battery pill reading the first battery from
  `/sys/class/power_supply`. It shows percent and a one-word state (`Chg`/`Dis`/
  `Full`/`AC`) and ticks every 30 seconds; clicking opens a popup with the
  percent, a fill bar and the time-to-full / time-to-empty when the kernel
  reports it.
- `statusbar`: a network pill that reads NetworkManager through `nmcli` and
  falls back to `networkctl` (systemd-networkd). It shows the connection state
  and, when connected, the Wi-Fi SSID or interface name, and ticks every 15
  seconds; clicking opens a popup with the state and interface.
- `statusbar`: a shared popup window used by the volume, brightness and battery
  pills, anchored under the bar so the panel floats the same way the bar does.

### Changed

- The statusbar inset is now 10 px margin and 10 px padding on every side,
  matching `gaps outer` in the compositor config the same way the sidebar and
  launcher already do, so the bar and the window rectangles use one measure on
  every exposed screen edge.
- The right-hand end of the bar is now a controls row of soft pills (volume,
  brightness, battery, network, logout) that reuse the same pill language as the
  RAM and CPU stat pills, so both ends of the bar read as one material.
- Every new pill is optional at runtime: it is only placed on the bar when its
  backend is present (a working `pactl`, a writable backlight device, a battery
  in `/sys/class/power_supply`, or a network manager), and disappears
  automatically on machines that lack it.

## [0.6.6] - 2026-10-14

### Changed

- Matched the bar gap to the compositor's `gaps outer` (6 px) rather than
  `gaps inner`, so the statusbar, sidebar and launcher all inset by the same
  measure the window rectangles use on every exposed screen edge.

## [0.6.3] - 2026-10-13

### Changed

- Aligned the widgets to the compositor's gaps: the statusbar, sidebar and toasts
  are now inset by `gaps outer` (10 px) on every exposed screen edge, the same
  measure as the window rectangles they sit beside.
- Squared off everything inside the widgets as well as their windows: the
  statusbar has no rounded corners, the workspace chips, the clock and the
  compositor, RAM and CPU readouts are all flat (no pill radius), and the sidebar
  and its toasts have squared-off cards.
- Fixed a quadrant mismatch in the sidebar window driver: the panel's own
  top margin was being shadowed by the shared expansion, so a driver that only
  set `the top margin` and expected the rest to be cleared came out off-center.
  The sidebars and toasts now set all four margins explicitly, and the panel keeps
  the `BAR_HEIGHT` gap above it instead of sitting in the bar's strip.

## [0.6.2] - 2026-10-13

### Changed

- Flipped the shared design back to squared-off, floating cards: the bar, the
  sidebar panel and the launcher panel now have no border and no rounded
  corners (radius 0), but they stay floating at `theme::GAP` off the screen edge.
  The workspace chips and the RAM/CPU pills keep their circles, so the bar still
  reads as a status line.

## [0.6.1] - 2026-10-13

### Changed

- README: document the CPU and memory pills, and add the missing `sidebar`
  startup line to the Hyprland example.

## [0.6.0] - 2026-10-13

### Added

- `statusbar`: CPU and memory readings at the right-hand end, replacing the
  compositor name badge. Both come from `/proc` — `meminfo` for the memory in
  use, and two `stat` samples two seconds apart for the CPU share — so the bar
  gains no dependency. Each reading is a soft pill matching the workspace chips.

### Changed

- The widgets are borderless. The bar's pill, the sidebar panel and the launcher
  drop their 1 px outlines; the panels become rounded cards (12 px) instead of
  squared-off 4 px, and buttons become soft pills like the workspace chips.
- `statusbar`: the pill is inset by `theme::GAP` (6 px) on every side, the same
  measure as `gaps outer`, and the 6 px padding keeps the circular workspace
  chips concentric with the pill's caps (pill radius = chip radius + padding).
- `sidebar`: the panel and its toasts float `theme::GAP` off the right and
  bottom edges instead of sitting flush against the screen.
- `shell-common`: `theme::BAR_HEIGHT` is 44 (the 32 px pill plus a `GAP` above
  and below it), and a new `theme::GAP` (6) names the screen-edge gap.

## [0.5.1] - 2026-10-13

### Fixed

- `make install` now copies all three binaries. The recipe expanded to
  `cp target/release/statusbar sidebar launcher <prefix>/`, so only the first
  source carried the `target/release/` prefix and `sidebar` and `launcher` were
  looked up relative to the working directory; the copy failed with "cannot
  stat" and left a stale binary in place.

## [0.5.0] - 2026-10-13

### Added

- Every widget is now set in [undefined](https://undefined-medium.com), the
  typeface sway, foot and kitty already use. The family is a single rule in the
  shared base sheet, so a widget that adds a label inherits it.
- `sidebar`: a notification that arrives while the panel is closed now shows as
  a toast in the top-right, below the statusbar, instead of waiting for `mod+n`.
  A toast expires after six seconds, a critical one stays until it is clicked
  away, and the newest four are kept. Toasts are in memory only; the panel's
  history file is unchanged.

### Changed

- `statusbar`: the bar is a floating pill inset from every screen edge and
  rounded past half its height, instead of a full-width strip with a rule under
  it. The workspace chips are circles — a filled accent for the focused
  workspace, a ring for an urgent one.
- `shell-common`: `theme::BAR_HEIGHT` is 40 (the 32 px pill plus its 4 px
  margins), the strip the compositor reserves and the sidebar docks below.

## [0.4.0] - 2026-10-13

### Added

- Every widget now hot-reloads the palette. `shell-common` keeps the
  `CssProvider` it styles the display with and watches
  `~/.config/widgets/palette.css` through a `GFileMonitor`; when `palette sync`
  rewrites the file the same provider is loaded with the rebuilt document and
  every open widget restyles in place, with no restart. A palette file that
  does not exist yet is still watched, so a widget started before its first
  `palette sync` picks the colors up as soon as the file appears.

## [0.3.1] - 2026-10-13

### Fixed

- `sidebar`: the daemon could not be started at all through the compositor
  binding. `Application::run` forwards the process command line to
  `GApplication`, which reads a bare word as a *file to open*; a widget that
  does not handle opening files aborts before `activate` runs, so
  `sidebar toggle` died with `GLib-GIO-CRITICAL: This application can not open
  files` and exit code 1, and `mod+n` never showed a panel. The three widgets
  now run through one `shell_common::app::run`, which gives the application the
  program name and nothing else. The same forwarded argument aborted
  `statusbar` and `launcher` when they were started with any extra word.
- `launcher`: the query line wore a blue border. The default theme paints a
  2px accent outline *inside* a focused entry and the launcher never reset it.
  The outline is gone and the rule under the query turns `palette_color4` while
  it has focus — the same accent as the focused workspace chip and the selected
  row. Buttons get the same reset in the shared base sheet, with a
  palette-coloured focus border for keyboard navigation.

## [0.3.0] - 2026-10-13

### Fixed

- `sidebar`: starting the daemon through the compositor binding left the panel
  hidden, so the first `mod+n` after login appeared to do nothing. The binding
  runs `sidebar toggle`, which started the daemon with no window presented;
  only a second press revealed the panel. A `toggle` that has to start the
  daemon now presents it, and the two entry points mean different things:
  `sidebar toggle` starts the daemon *and* opens the panel, a bare `sidebar`
  primes the daemon without covering the screen.
- `statusbar`: the Hyprland workspace list never followed a workspace change.
  Events on `.socket2.sock` are written in lower case (`workspace>>2`) while
  the client matched upper-case literals (`"WORKSPACE"`, `"WORKSPACEV2"`, …),
  so no event was ever recognised: the bar drew the workspaces it saw at
  startup and then stood still. The match is now case-insensitive over the
  workspace-related events in Hyprland's own event table.
- `statusbar`: the sway workspace list could stop updating after a workspace
  switch. Replies and subscribed events share the i3 IPC socket and the client
  read whichever frame came next, so an event arriving while a reply was
  pending was parsed as a workspace list and killed the reader thread, with
  nothing on screen to say so. Frames are now classified by the event bit.
- `sidebar`: a stale PID file could make `toggle` signal an unrelated process.
  Liveness alone was checked and PIDs are recycled; the file is now trusted
  only while `/proc/<pid>/comm` still names the widget.

### Changed

- `shell-common`: one single-instance guard (`shell_common::instance`) behind
  the sidebar and the statusbar, replacing the sidebar's private PID-file
  logic. `statusbar` is now a singleton, so a compositor that starts it twice
  keeps one bar instead of two in the same exclusive zone.
- The three widgets were redesigned on one Swiss-typographic scale: a single
  spacing scale (4/8/12/16/24), hairlines instead of boxes, type size and
  weight instead of fills for hierarchy, one accent for focus and one for
  urgency, and flat squared-off surfaces.
  `shell_common::theme::BAR_HEIGHT` now holds the strip the bar reserves, so
  the sidebar starts below it instead of covering it.
- `sidebar`: `GetCapabilities` advertises only `body` — the invented
  `persistence` capability is gone — and `GetServerInformation` reports the
  crate version.
- Tests now cover what used to be checked by eye: the sidebar panel's list and
  actions (run against a real GTK display), the sway frame reader, the
  Hyprland event filter, the niri event application, and the new
  single-instance guard.

## [0.2.2] - 2026-10-13

### Fixed

- musl builds now work with rustup's toolchain, not just a distribution one.
  Every musl Rust target defaults to `+crt-static` (RFC 1721), which asks the
  linker for the *static* build of GTK4 and gtk4-layer-shell, so the build died
  at the link step with `ld: cannot find -lglib-2.0` and `ld: have you installed
  the static version of the glib-2.0 library ?` — advice that cannot be taken,
  because Alpine and Void ship those libraries as shared objects only. The
  repository now carries a `.cargo/config.toml` that sets
  `-C target-feature=-crt-static` for `cfg(target_env = "musl")`, so the system
  libraries link dynamically the way they already do on glibc. The rule is
  architecture independent and covers x86_64, aarch64, armv7 and riscv64 musl
  targets, and it never matches a glibc target.
- `shell-common`: the stylesheet check validates the widget sheets on every
  run. It used to return early with "no display available" unless a compositor
  was present, which meant CI passed it without parsing anything. GTK parses
  CSS without a display, so the check is now unconditional and also asserts
  that GTK still reports the CSS it rejects — otherwise it could go back to
  passing vacuously without anyone noticing. All GTK calls stay in one test:
  GTK is not thread safe, and two tests parsing at once report errors that are
  not there.

## [0.2.1] - 2026-10-13

### Fixed

- Release builds no longer get OOM-killed on memory-constrained machines. The
  release profile used fat LTO (`lto = true`); linking the sidebar — the largest
  binary, pulling in zbus, tokio and GTK4 — was killed with `SIGKILL` and
  surfaced as `error: could not compile \`sidebar\``. `lto = "thin"` keeps
  cross-crate optimization with a far lower memory ceiling.

## [0.2.0] - 2026-10-13

### Added

- Builds and runs on musl: the workspace compiles for
  `x86_64-unknown-linux-musl` (Alpine) with `gtk4.0-dev` +
  `gtk4-layer-shell-dev`, alongside the glibc builds.

### Changed

- `shell-common`: a single `theme::init(<widget style.css>)` per widget builds
  one stylesheet document out of the palette, a shared base sheet
  (`shell_common::theme::BASE_CSS`: transparent windows, panel surfaces, ghost
  buttons, muted text) and the widget's own CSS, so the three widgets share one
  visual language instead of repeating the same rules.
- Widget stylesheets live next to their crates (`crates/<widget>/style.css`).
- Every crate takes its version from the workspace (`version.workspace = true`).

## [0.1.0] - 2026-10-13

### Added

- `statusbar`: top bar with workspace buttons driven by the compositor's own
  IPC (sway, niri and Hyprland), a clock refreshed every 30 seconds and a
  compositor badge.
- `sidebar`: notification daemon and overlay list, toggled with `mod+n`.
- `launcher`: centered application launcher listing installed desktop
  entries, opened with `mod+space`.
- `shell-common`: the shared library behind those three. It loads
  `~/.config/widgets/palette.css` (`@define-color palette_bg`, `palette_fg`,
  `palette_cursor` and `palette_color0`..`palette_color15`, generated by
  `palette sync`) with built-in fallbacks, and builds layer-shell windows
  through one `LayerWindow` builder.

Every binary answers `--version` before it opens a display, and exits with a
clear message when the compositor does not implement wlr-layer-shell.
