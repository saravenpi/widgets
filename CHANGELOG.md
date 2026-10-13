# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
