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
| `statusbar` | `statusbar` | Top bar: workspaces via the compositor's own IPC (sway, niri, Hyprland), a clock, CPU/memory readings, and optional volume, brightness, battery and network pills plus a logout button |
| `sidebar` | `sidebar` | `org.freedesktop.Notifications` daemon plus an overlay list; `mod+n` toggles it |
| `launcher` | `launcher` | Centered dmenu-style launcher over installed `.desktop` entries; `mod+space` |
| `exit` | `exit` | Centered session control dialog (exit, reboot, poweroff); `mod+Shift+e` |
| `settings` | `settings` | Display resolution and system settings dialog; `mod+s` |
| `shell-common` | (library) | Palette CSS loading, YAML config loader (`~/.config/widgets/config.yml`), and shared layer-shell boilerplate |

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
- **Floating, squared-off, borderless** — no borders and no shadows anywhere, and
  nothing inside the widgets is rounded: the bar is a floating bar, the sidebar
  and launcher are floating cards, and the workspace chips, the clock, the
  compositor, the RAM and the CPU readings are all flat pieces. Every widget keeps
  its modules on a hard grid, square off everything, and aligns to the compositor's
  gaps (`shell_common::theme::GAP`) on the screen edge. The bar insets itself 10 px
  from every screen edge and keeps 10 px of padding inside, the same measure as
  `gaps outer` in the compositor config.
- **Toasts, not requests.** A notification that arrives while the panel is shut
  shows itself in the top-right rather than waiting for `mod+n`.

The statusbar reserves a 52 px strip at the top (`shell_common::theme::BAR_HEIGHT`),
the 32 px bar plus a `GAP` above and below it (matching `gaps outer` in the compositor
config); the sidebar and the toasts start below it instead of covering the bar's
right-hand end.

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

The statusbar's control icons are bitmaps rather than stylesheet rules, so they
cannot follow the cascade the way the labels beside them do. The bar draws them
again whenever the stylesheet is rebuilt, so a `palette sync` moves the glyphs
and the text together instead of leaving the icons a release behind.

## Running

Start the bar from your compositor; all three binaries live in `~/.local/bin`.

```text
# sway      ~/.config/sway/config
exec $HOME/.local/bin/statusbar
exec $HOME/.local/bin/sidebar         # owns org.freedesktop.Notifications
bindsym $mod+n      exec $HOME/.local/bin/sidebar toggle
bindsym $mod+space  exec $HOME/.local/bin/launcher
bindsym $mod+s      exec $HOME/.local/bin/settings
bindsym $mod+Shift+e exec $HOME/.local/bin/exit

# niri      ~/.config/niri/config.kdl
spawn-at-startup "sh" "-c" "$HOME/.local/bin/statusbar"

# Hyprland  ~/.config/hypr/hyprland.conf
exec-once = $HOME/.local/bin/statusbar
exec-once = $HOME/.local/bin/sidebar   # owns org.freedesktop.Notifications
bind = $mainMod, N, exec, $HOME/.local/bin/sidebar toggle
bind = $mainMod, Space, exec, $HOME/.local/bin/launcher
bind = $mainMod, S, exec, $HOME/.local/bin/settings
bind = $mainMod SHIFT, E, exec, $HOME/.local/bin/exit
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

The right-hand end carries the CPU and memory load as two pills, then a row of
optional control pills: volume, brightness, battery, network and a logout button.
Each control pill leads with a pixel-art glyph — the
[pixelarticons][pixelarticons] set, compiled into the binary as 24×24 masks and
drawn in the palette foreground, so the bar takes on no icon dependency and no
image loader: the SVG bodies they were rasterized from need librsvg, which is an
optional package on most distributions, so a bar built on a machine without it
used to draw a pill's label and nothing beside it. Each control pill only appears
when its backend is present, so the bar
looks right on a desktop without a battery or on a machine without audio. Memory is
`MemTotal - MemAvailable` from `/proc/meminfo`; the CPU share is the busy delta
between two `/proc/stat` samples two seconds apart. Both are read from `/proc`
with no dependency, and a reading that cannot be taken leaves its pill empty
rather than showing a stale number.

- **Volume** reads the default sink through `pactl` (PulseAudio / PipeWire). Click
  the pill to open a popup with the percent, a fill bar and `+1%`/`-1%` steps; the
  pill updates after each change.
- **Brightness** reads the first backlight device from `/sys/class/backlight` and
  writes it back. Click for a popup with the percent, a fill bar and `+5%`/`-5%`
  steps.
- **Battery** reads the fullest battery from `/sys/class/power_supply` and ticks
  every 30 seconds. It shows percent and a one-word state: `Chg` while current is
  flowing in, `Full` at the top, `AC` on external power with the battery held back
  — a charge threshold, usually — and `Dis` on its own cells. Click for a popup
  with the percent, a fill bar and the time-to-full / time-to-empty when the
  kernel reports it and the battery is actually moving.
- **Network** reads NetworkManager through `nmcli`, falls back to
  `networkctl` (systemd-networkd), and falls back again to the kernel's own
  interfaces under `/sys/class/net`. A minimal install — Alpine's default, or a
  virtual machine with a single virtio link — runs no network manager at all, and
  the pill used to be left out of the row entirely on those machines. It shows the
  connection state and, when connected, the Wi-Fi SSID or interface name, and
  ticks every 15 seconds; click for a popup with the state and interface.
- **Logout** ends the current session through
  `loginctl terminate-session $XDG_SESSION_ID` — the same action a compositor
  logout binding (`mod+shift+e`) should map to — and carries the power glyph. The
  bar only places it when `loginctl` can describe the session, and asking that is
  a read: the button never ends the session just to prove it could. It is hidden
  when the session id is unknown or `loginctl` is absent.

### When something is missing

A control pill is placed only when its backend answers, and a pill whose icon
cannot be painted simply omits the icon, so a bar with a gap and a bar whose
glyphs never arrived look the same from outside. `statusbar --diagnose` prints
which one it is — every control, whether it is in the row and why or why not,
whether each glyph reaches a texture, the compositor, and whether the machine is
virtualised:

```sh
statusbar --diagnose
```

```text
statusbar 0.10.0
compositor: Sway
machine: virtual (QEMU Standard PC (Q35 + ICH9, 2009))
display: wayland
GSK_RENDERER: unset, GTK chooses
pills
  volume     absent  no pactl, or nothing for it to answer for (pulseaudio-utils, pipewire-pulse)
  brightness absent  nothing writable in /sys/class/backlight (a virtual machine has none)
  battery    absent  no BAT device in /sys/class/power_supply (a virtual machine has none)
  network    placed  a link answers
  logout     absent  $XDG_SESSION_ID is unset, or loginctl is missing (elogind)
  the network pill reads: Net On eth0
glyphs
  battery    24×24
  brightness 24×24
  volume     24×24
  network    24×24
  logout     24×24
```

It reads and reports; it never draws a window, so it is safe to run next to a bar
that is already up. Without a display it says so instead of guessing at the
glyphs.

### Virtual machines

A VM is where the widgets are most likely to look wrong, and for two reasons that
are worth telling apart.

**The bar has fewer pills, by design.** A virtual machine has no battery, no
backlight, and often no audio server or network manager, so those pills are left
out rather than drawn dead. `--diagnose` above names each one. On a minimal
Alpine guest, expect a row of only what is actually there.

**GTK4 needs OpenGL 3.3, and a virtual GPU may not have it.** Since UTM 4.7.5
(QEMU 10) the `virtio-gpu` display exposes OpenGL **2.1**, below what GTK4's NGL
renderer requires, which shows up as missing or corrupted window content rather
than as an error ([utmapp/UTM#7691][utm-gl]). A bar whose compositor renders in
software is not exempt: `WLR_RENDERER=pixman` settles how *sway* draws, not how
GTK draws. Start the widgets with one of these, which forces a renderer that does
not need 3.3:

```sh
GSK_RENDERER=cairo statusbar          # GTK's CPU renderer
LIBGL_ALWAYS_SOFTWARE=1 statusbar     # Mesa's llvmpipe, which reports 3.3+
```

Either works for a shell widget at this size, and both are plain environment
variables, so put the one that works in the compositor's exec line. On the host,
turning 3D acceleration off for the VM is the other end of the same fix.

[utm-gl]: https://github.com/utmapp/UTM/issues/7691

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

[pixelarticons]: https://icon-sets.iconify.design/pixelarticons/
