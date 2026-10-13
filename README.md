# widgets

Wayland desktop widgets: a status bar, a notification sidebar and an
application launcher. Rust + GTK4 + gtk4-layer-shell, colored by
[palette](https://github.com/Saravenpi/palette).

Works on **sway**, **niri** and **Hyprland** — anything that implements the
wlr-layer-shell protocol. Wayland only: on X11 or in a bare TTY each binary
prints a clear message and exits without opening a window
(`gtk4_layer_shell::is_supported()`).

## Crates

| Crate | Binary | What it does |
| --- | --- | --- |
| `statusbar` | `statusbar` | Top bar: workspaces via the compositor's own IPC (sway, niri, Hyprland), a clock and CPU/memory readings |
| `sidebar` | `sidebar` | `org.freedesktop.Notifications` daemon plus an overlay list; `mod+n` toggles it |
| `launcher` | `launcher` | Centered dmenu-style launcher over installed `.desktop` entries; `mod+space` |
| `shell-common` | (library) | Palette CSS loading and the shared layer-window / theme boilerplate |

Every binary answers `--version` before it opens a display, so an installer can
verify it without a compositor.

## Requirements

| Requirement | Why | Minimum |
| --- | --- | --- |
| Wayland compositor with wlr-layer-shell | the windows are layer surfaces | sway, niri or Hyprland |
| GTK4 development files | the `gtk4` crate is built against them | **4.10** (matches the `v4_10` feature) |
| gtk4-layer-shell development files | layer-surface support | C library 1.0+ (crate `gtk4-layer-shell` 0.8) |
| the `undefined` font | the typeface every widget is set in | [undefined-medium.com](https://undefined-medium.com), `fonts` package |
| Rust + Cargo | the build | 1.85+ (pinned by `gtk4-layer-shell-sys`) |
| pkg-config and a C toolchain | the `-sys` crates use them | any |

GTK 4.10 is a hard floor: older versions fail the version check at build time.

### Package names per distribution

Install the GTK4 development files and the gtk4-layer-shell ones together. Some
distributions make the layer-shell package depend on the GTK4 one and some do
not — Alpine 3.24 is in the second group, so it needs `gtk4.0-dev` spelled out.
Arch, Gentoo and Nix ship the headers in the main package.

| Distribution | Install |
| --- | --- |
| Debian 13 (trixie) and newer | `sudo apt install libgtk-4-dev libgtk4-layer-shell-dev` |
| Ubuntu 24.10 and newer | `sudo apt install libgtk-4-dev libgtk4-layer-shell-dev` |
| Arch, Manjaro, EndeavourOS | `sudo pacman -S gtk4 gtk4-layer-shell` |
| Fedora | `sudo dnf install gtk4-devel gtk4-layer-shell-devel` |
| openSUSE Tumbleweed | `sudo zypper install gtk4-devel gtk4-layer-shell-devel` |
| Void Linux | `sudo xbps-install gtk4-devel gtk4-layer-shell-devel` |
| Alpine | `sudo apk add gtk4.0-dev gtk4-layer-shell-dev` |
| Gentoo | `sudo emerge gui-libs/gtk gui-libs/gtk4-layer-shell` |
| Nix / NixOS | `nix-shell -p gtk4 gtk4-layer-shell pkg-config cargo` |

Install a Rust toolchain if the distribution has none (`rustup`, or the
`rust` + `cargo` packages), plus `build-base` / `build-essential` / `base-devel`
for the linker.

**Older releases.** Debian 12 (bookworm) ships GTK 4.8 and Ubuntu 24.04 ships
GTK 4.14 without a packaged `gtk4-layer-shell`, so neither works out of the box:
bookworm is below the GTK floor, and 24.04 needs the layer-shell library built
from source. Build it once with Meson and the build will find it (needs
`meson`, `ninja` and `gobject-introspection`):

```sh
git clone https://github.com/wmww/gtk4-layer-shell
cd gtk4-layer-shell
meson setup -Dprefix=/usr/local build
meson compile -C build
sudo meson install -C build
```

### Alpine, Void and other musl systems

Alpine's Rust target is musl, so a native build needs no cross-compilation:

```sh
sudo apk add build-base pkgconf gtk4.0-dev gtk4-layer-shell-dev rust cargo
```

**rustup's musl toolchain is the one trap, and this repository already handles
it.** Every musl Rust target defaults to `+crt-static` ([RFC 1721][rfc1721]),
which asks the linker for the *static* build of the C runtime and of every
system library. Alpine and Void only package shared GTK4 and gtk4-layer-shell
libraries, so the build stops at the link step:

```text
ld: cannot find -lglib-2.0: No such file or directory
ld: have you installed the static version of the glib-2.0 library ?
...
error: could not compile `sidebar` (bin "sidebar") due to 1 previous error
```

That hint cannot be followed: `glib-2.0`, `intl`, `cairo`, `gtk-4` and the rest
are only packaged as shared objects (`libgtk-4.a` does not exist on Alpine).
The checkout carries a `.cargo/config.toml` that turns the default off:

```toml
[target.'cfg(target_env = "musl")']
rustflags = ["-C", "target-feature=-crt-static"]
```

Cargo reads it on its own as long as the build runs from the repository root
(`make`, `facile`, a plain `cargo build`), so there is nothing to export. The
rule matches every musl architecture — `x86_64`, `aarch64` (Raspberry Pi, ARM
laptops), `armv7`, `riscv64` — and never matches glibc, so `gnu` builds are
untouched. The system libraries are then linked dynamically, exactly as they
are on glibc.

That means the binaries link against the system's musl libc and GTK: they run
on the machine that built them, not in a `FROM scratch` image. To force a fully
static build instead, set `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` — the
environment takes precedence over the file above.

[rfc1721]: https://rust-lang.github.io/rfcs/1721-crt-static.html

## Build

```sh
make build     # cargo build --release (all three binaries)
make check     # cargo check --workspace
make install   # copy the binaries to ~/.local/bin (override with PREFIX=...)
make clean
```

The same thing without make:

```sh
cargo build --release
install -Dm755 target/release/statusbar target/release/sidebar \
                     target/release/launcher ~/.local/bin/
```

The release profile uses `lto = "thin"` and `strip = true`.

## Install with facile

The repository carries a `facile.toml`, so it can be installed and updated as a
single catalog entry that builds all three binaries:

```sh
facile install widgets
facile update widgets
facile uninstall widgets
```

## Design

One shared stylesheet (`shell_common::theme::BASE_CSS`) defines what the three
widgets agree on, so each widget's own sheet only says what makes it different.
It follows a single Swiss-typographic scale rather than a per-widget look:

- **One spacing scale** — 4, 8, 12, 16, 24 px. Space separates things; borders
  do not.
- **Hairlines, not boxes.** One 1 px rule in a low-contrast tone marks a
  division: under the bar, under the panel title, between notifications.
- **Type carries the hierarchy** — size and weight (500/600) rather than fills,
  shadows or gradients. Nothing is decorated for its own sake. Everything is set
  in [undefined](https://undefined-medium.com).
- **One accent.** `palette_color4` marks the focused workspace and the selected
  launcher row; `palette_color1` is reserved for urgency.
- **Floating, rounded, borderless** — no borders and no shadows anywhere. The
  statusbar is a pill and the sidebar and launcher are rounded cards (12 px),
  each inset from the screen edge by the same gap as `gaps outer` in the
  compositor config (`shell_common::theme::GAP`, 6 px). The bar's circular
  workspace chips are cut to the pill's cap: their radius is the pill radius
  minus the bar's padding (`r_outer = r_chip + padding`), so they sit
  concentric with it.
- **Toasts, not requests.** A notification that arrives while the panel is shut
  shows itself in the top-right rather than waiting for `mod+n`.

The statusbar reserves a 44 px strip at the top (`shell_common::theme::BAR_HEIGHT`),
the 32 px pill plus a `GAP` above and below it; the sidebar and the toasts start
below it instead of covering the bar's right-hand end.

## Colors

Every widget loads `~/.config/widgets/palette.css` at startup, generated by
`palette sync`. It defines `@palette_bg`, `@palette_fg`, `@palette_cursor` and
`@palette_color0`..`@palette_color15` as GTK named colors; the widget
stylesheets reference those tokens. If the file is missing, built-in defaults
keep the widgets running, so palette is optional but recommended:

```sh
palette sync
```

The widgets watch that file, so a running bar, sidebar or launcher restyles
itself the moment `palette sync` rewrites it — no restart, no compositor reload
required. A file that does not exist yet is watched too, so a widget started
before its first sync picks the colors up when it appears.

## Running

Start the bar from your compositor; all three binaries live in `~/.local/bin`.

```text
# sway      ~/.config/sway/config
exec $HOME/.local/bin/statusbar
exec $HOME/.local/bin/sidebar         # owns org.freedesktop.Notifications
bindsym $mod+n      exec $HOME/.local/bin/sidebar toggle
bindsym $mod+space  exec $HOME/.local/bin/launcher

# niri      ~/.config/niri/config.kdl
spawn-at-startup "sh" "-c" "$HOME/.local/bin/statusbar"

# Hyprland  ~/.config/hypr/hyprland.conf
exec-once = $HOME/.local/bin/statusbar
bind = $mainMod, N, exec, $HOME/.local/bin/sidebar toggle
bind = $mainMod, Space, exec, $HOME/.local/bin/launcher
```

### The statusbar

`statusbar` is a singleton. It claims a PID file in `$XDG_RUNTIME_DIR` before it
draws anything, so starting it twice — a compositor reload, a stray `exec` line
next to a `bindsym` — leaves one bar on screen: the second copy prints
`already running (pid …)` and exits without touching the display. A PID file
whose process is gone, or whose process is no longer a `statusbar`, is treated
as stale and taken over, so a crash never locks the bar out of its own slot.

Workspaces come from the compositor's own IPC and follow it live: the event
stream for **sway** (i3 protocol) and **niri**, and `.socket2.sock` for
**Hyprland**. Clicking a chip focuses that workspace through the same IPC.

### The sidebar

`sidebar` is a notification daemon: it claims `org.freedesktop.Notifications` on
the session bus and keeps the last 50 notifications in
`$XDG_STATE_HOME/widgets/notifications.json` (`~/.local/state/widgets/…`).
Press `Esc` or click `x` to dismiss a notification, `Clear all` to empty the list.

Because it owns the bus name, it has to be running for notifications to arrive
at all — start it from the compositor (the bare `sidebar` line above) rather than
waiting for the first `mod+n`. Only one process can hold
`org.freedesktop.Notifications`, so **do not run mako alongside it**: the second
daemon fails to claim the name and its notifications go to whichever one got
there first.

A notification that arrives while the panel is closed is not lost. It shows as a
toast in the top-right, below the statusbar, for six seconds — critical ones stay
until dismissed — and it lands in the history either way. Click a toast to
dismiss it; the panel lists everything that arrived, toasts or not.

The two ways to start it mean different things:

```sh
sidebar          # prime the daemon; the panel stays out of the way
sidebar toggle   # start the daemon if needed, and show the panel
```

`sidebar toggle` is the compositor binding, so with no daemon running it starts
one **and presents the panel** — the first `mod+n` after login shows your
notifications rather than silently starting a hidden window. Every later
`toggle` finds the running daemon through a PID file in `$XDG_RUNTIME_DIR` and
sends it `SIGUSR1` to show or hide the panel.

### The launcher

`launcher` reads `.desktop` entries from `$XDG_DATA_HOME/applications`,
`~/.local/share/applications` and `/usr/share/applications`, hides entries
marked `NoDisplay`/`Hidden`, and shows the best 10 matches. Type to filter,
`Up`/`Down` to move, `Enter` to launch, `Esc` to close.

## License

MIT
