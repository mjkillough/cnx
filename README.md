# Cnx — [![Build Status](https://travis-ci.org/mjkillough/cnx.svg?branch=master)](https://travis-ci.org/mjkillough/cnx)

A simple bar for use with simple WMs.

Cnx doesn't rely on functionality from any specific WM, instead preferring to get its data from generic properties defined in EWMH. If your WM implements enough of EWMH, it should work with Cnx.


## Features

Cnx is written to be customisable, simple and fast.

Where possible, it prefers to asynchronously wait for changes in the underlying data sources (and uses `mio`/`tokio` to achieve this), rather than periodically calling out to external programs.

There are currently these widgets available:
 - Active Window Title — Shows the title (EWMH's `_NET_WM_NAME`) for the currently focused window (EWMH's `_NEW_ACTIVE_WINDOW`).
 - Pager — Shows the WM's workspaces/groups, highlighting whichever is currently active. (Uses EWMH's `_NET_DESKTOP_NAMES`/`_NET_NUMBER_OF_DESKTOPS`/`_NET_CURRENT_DESKTOP`).
 - Sensors — Periodically parses and displays the output of the `sensors` utility, allowing CPU temperature to be displayed.
 - Volume — Uses `alsa-lib` to show the current volume/mute status of the default output device.
 - Battery — Uses `/sys/class/power_supply/` to show details on the remaining battery and charge status.
 - Clock — Shows the time.


## Installing

Your system must first have all of the required [dependencies](#dependencies)

To accept the default widgets/configuration, you can install and run using:

```
cargo install cnx
cnx
```

However, this is probably not what you want. You should either clone this repository and modify `src/bin/cnx.rs` to your liking, or (preferably) make a new binary project which depends on `cnx`. The code in `src/bin/cnx.rs` should give you an idea of what to do in your binary project.


## Dependencies

In addition to the Rust dependencies in `Cargo.toml`, Cnx also depends on these system libraries:
 - `x11-xcb`
 - `xcb-util`: `xcb-ewmh` / `xcb-icccm` / `xcb-keysyms`
 - `alsa-lib`
 - `pango`
 - `cairo`
 - `pangocairo`

The following Ubuntu packages should allow your system to meet these requirements:

```
apt-get install libx11-xcb-dev libxcb-ewmh-dev libasound2-dev libpango1.0-dev libcairo2-dev libpangocairo-1.0-0
```


## Tests

Unfortunately there aren't many. You can run what's here with:

```
cargo test
```


## License

MIT
