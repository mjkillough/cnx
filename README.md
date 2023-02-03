# Cnx — [![CI](https://github.com/mjkillough/cnx/actions/workflows/ci.yml/badge.svg)](https://github.com/mjkillough/cnx/actions)

A simple X11 status bar for use with simple WMs.

Cnx doesn't rely on functionality from any specific WM, instead preferring to
get its data from generic properties defined in EWMH. If your WM implements
enough of EWMH, it should work with Cnx.

![screenshot of cnx](/screenshot.png?raw=true)

## Features

Cnx is written to be customisable, simple and fast.

Where possible, it prefers to asynchronously wait for changes in the underlying
data sources (and uses [`tokio`] to achieve this), rather than periodically
calling out to external programs.

[`tokio`]: https://tokio.rs/

There are currently these widgets available:

 - Active Window Title — Shows the title (EWMH's `_NET_WM_NAME`) for
   the currently focused window (EWMH's `_NEW_ACTIVE_WINDOW`).
 - Pager — Shows the WM's workspaces/groups, highlighting whichever is
   currently active. (Uses EWMH's `_NET_DESKTOP_NAMES`,
   `_NET_NUMBER_OF_DESKTOPS` and `_NET_CURRENT_DESKTOP`).
 - Clock — Shows the time.

The cnx-contrib crate contains additional widgets:

- **Sensors** — Periodically parses and displays the output of the
  sensors provided by the system.
- **Volume** - Shows the current volume/mute status of the default output
  device.
- **Battery** - Shows the remaining battery and charge status.
- **Wireless** - Shows the wireless strength of your current network.
- **CPU** - Shows the current CPU consumption
- **Weather** - Shows the Weather information of your location
- **Disk Usage** - Show the current usage of your monted filesystem
- **LeftWM** - Shows the monitors and tags from LeftWM

The [`Sensors`], [`Volume`] and [`Battery`] widgets require platform
support. They currently support Linux (see dependencies below) and OpenBSD.
Support for additional platforms should be possible.

## How to use

Cnx is a library that allows you to make your own status bar.

In normal usage, you will create a new binary project that relies on the `cnx`
crate, and customize it through options passed to the main `Cnx` object and
its widgets. (It's inspired by [`QTile`] and [`dwm`], in that the configuration
is done entirely in code, allowing greater extensibility without needing complex
configuration handling).

[`QTile`]: http://www.qtile.org/
[`dwm`]: http://dwm.suckless.org/

An simple example of a binary using Cnx is:

```rust
use cnx::text::*;
use cnx::widgets::*;
use cnx::{Cnx, Position};

fn main() -> Result<()> {
    let attr = Attributes {
        font: Font::new("Envy Code R 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };

    let mut cnx = Cnx::new(Position::Top);
    cnx.add_widget(ActiveWindowTitle::new(attr.clone()));
    cnx.add_widget(Clock::new(attr.clone()));
    cnx.run()?;

    Ok(())
}
```

A more complex example is given in [`src/bin/cnx.rs`] alongside the project.
(This is the default `[bin]` target for the crate, so you _could_ use it by
either executing `cargo run` from the crate root, or even running `cargo install
cnx; cnx`. However, neither of these are recommended as options for customizing
Cnx are then limited).

Before running Cnx, you'll need to make sure your system has the required
[dependencies].

[`src/bin/cnx.rs`]: https://github.com/mjkillough/cnx/blob/master/src/bin/cnx.rs
[dependencies]: #dependencies

## Dependencies

In addition to the Rust dependencies in `Cargo.toml`, Cnx also depends on these
system libraries:
 - `x11-xcb`
 - `xcb-util`: `xcb-ewmh` / `xcb-icccm` / `xcb-keysyms`
 - `pango`
 - `cairo`
 - `pangocairo`

The following Ubuntu packages should allow your system to meet these
requirements:

```
apt-get install libx11-xcb-dev libxcb-ewmh-dev libpango1.0-dev libcairo2-dev
```

If the `volume` feature is enabled (and it is by default), you will
also need `alsa-lib` on Linux:

```
apt-get install libasound2-dev
```

If the `wireless` feature is enabled (and it is not by default), you will also need `iwlib-dev`
on Linux:

```
apt install libiw-dev
```

## Tests

Unfortunately there aren't many. You can run what's here with:

```
cargo test
```


## License

MIT
