# Cnx — [![Build Status](https://travis-ci.org/mjkillough/cnx.svg?branch=master)](https://travis-ci.org/mjkillough/cnx)

A simple X11 status bar for use with simple WMs.

Cnx doesn't rely on functionality from any specific WM, instead preferring to
get its data from generic properties defined in EWMH. If your WM implements
enough of EWMH, it should work with Cnx.

![screenshot of cnx](/screenshot.png?raw=true)

## Features

Cnx is written to be customisable, simple and fast.

Where possible, it prefers to asynchronously wait for changes in the underlying
data sources (and uses [`mio`]/[`tokio`] to achieve this), rather than periodically
calling out to external programs.

[`mio`]: https://docs.rs/mio
[`tokio`]: https://tokio.rs/

There are currently these widgets available:
 - Active Window Title — Shows the title (EWMH's `_NET_WM_NAME`) for the
   currently focused window (EWMH's `_NEW_ACTIVE_WINDOW`).
 - Pager — Shows the WM's workspaces/groups, highlighting whichever is currently
   active. (Uses EWMH's
   `_NET_DESKTOP_NAMES`/`_NET_NUMBER_OF_DESKTOPS`/`_NET_CURRENT_DESKTOP`).
 - Sensors — Periodically parses and displays the output of the `lm_sensors`
   utility, allowing CPU temperature to be displayed.
 - Volume — Uses `alsa-lib` to show the current volume/mute status of the
   default output device. (Disable by removing default feature
   `volume-widget`).
 - Battery — Uses `/sys/class/power_supply/` to show details on the remaining
   battery and charge status.
 - Clock — Shows the time.

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
use cnx::*;

fn main() -> Result<()> {
    let attr = Attributes {
        font: Font::new("SourceCodePro 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };

    let mut cnx = Cnx::new(Position::Bottom)?;
    cnx_add_widget!(cnx, ActiveWindowTitle::new(&cnx, attr.clone()));
    cnx_add_widget!(cnx, Clock::new(&cnx, attr.clone()));
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

If the `volume-widget` feature is enabled (and it is by default), you will
also need `alsa-lib`:

```
apt-get install libasound2-dev
```


## Tests

Unfortunately there aren't many. You can run what's here with:

```
cargo test
```


## License

MIT
