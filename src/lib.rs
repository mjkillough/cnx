//! A simple X11 status bar for use with simple WMs.
//!
//! Cnx is written to be customisable, simple and fast. Where possible, it
//! prefers to asynchronously wait for changes in the underlying data sources
//! (and uses [`mio`]/[`tokio`] to achieve this), rather than periodically
//! calling out to external programs.
//!
//! # How to use
//!
//! Cnx is a library that allows you to make your own status bar.
//!
//! In normal usage, you will create a new binary project that relies on the
//! `cnx` crate, and customize it through options passed to the main [`Cnx`]
//! object and its widgets. (It's inspired by [`QTile`] and [`dwm`], in that the
//! configuration is done entirely in code, allowing greater extensibility
//! without needing complex configuration handling).
//!
//! An simple example of a binary using Cnx is:
//!
//! ```no_run
//! use anyhow::Result;
//!
//! use cnx::text::*;
//! use cnx::widgets::*;
//! use cnx::{Cnx, Position};
//!
//! fn main() -> Result<()> {
//!     let attr = Attributes {
//!         font: Font::new("Envy Code R 21"),
//!         fg_color: Color::white(),
//!         bg_color: None,
//!         padding: Padding::new(8.0, 8.0, 0.0, 0.0),
//!     };
//!
//!     let mut cnx = Cnx::new(Position::Top);
//!     cnx.add_widget(ActiveWindowTitle::new(attr.clone()));
//!     cnx.add_widget(Clock::new(attr.clone()));
//!     cnx.run()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! A more complex example is given in [`src/bin/cnx.rs`] alongside the project.
//! (This is the default `[bin]` target for the crate, so you _could_ use it by
//! either executing `cargo run` from the crate root, or even running `cargo
//! install cnx; cnx`. However, neither of these are recommended as options for
//! customizing Cnx are then limited).
//!
//! Before running Cnx, you'll need to make sure your system has the required
//! dependencies, which are described in the [`README`][readme-deps].
//!
//! # Built-in widgets
//!
//! There are currently these widgets available:
//!
//! - [`Active Window Title`] — Shows the title ([`EWMH`]'s `_NET_WM_NAME`) for
//!   the currently focused window ([`EWMH`]'s `_NEW_ACTIVE_WINDOW`).
//! - [`Pager`] — Shows the WM's workspaces/groups, highlighting whichever is
//!   currently active. (Uses [`EWMH`]'s `_NET_DESKTOP_NAMES`,
//!   `_NET_NUMBER_OF_DESKTOPS` and `_NET_CURRENT_DESKTOP`).
//! - [`Sensors`] — Periodically parses and displays the output of the
//!   sensors provided by the system.
//! - [`Volume`] - Shows the current volume/mute status of the default output
//!   device.
//! - [`Battery`] - Shows the remaining battery and charge status.
//! - [`Clock`] — Shows the time.
//!
//! The [`Sensors`], [`Volume`] and [`Battery`] widgets require platform
//! support. They currently support Linux (see dependencies below) and OpenBSD.
//! Support for additional platforms should be possible.
//!
//! # Dependencies
//!
//! In addition to the Rust dependencies in `Cargo.toml`, Cnx also depends on
//! these system libraries:
//!
//!  - `xcb-util`: `xcb-ewmh` / `xcb-icccm` / `xcb-keysyms`
//!  - `x11-xcb`
//!  - `pango`
//!  - `cairo`
//!  - `pangocairo`
//!
//! Some widgets have additional dependencies on Linux:
//!
//!  - [`Volume`] widget relies on `alsa-lib`
//!  - [`Sensors`] widget relies on [`lm_sensors`] being installed.
//!
//! # Creating new widgets
//!
//! Cnx is designed such that thirdparty widgets can be written in external
//! crates and used with the main [`Cnx`] instance. However, I've never done
//! this.
//!
//! The adventurous may choose to implement the [`Widget`] trait and see how
//! far they can get. The [`Widget`] implementation can assume it's being run
//! from a single-threaded [`tokio`] event-loop, but this is an implementation
//! detail that should not be relied upon.
//!
//! The built-in [`widgets`] should give you some examples on which to base
//! your work.
//!
//! [`mio`]: https://docs.rs/mio
//! [`tokio`]: https://tokio.rs/
//! [`Cnx`]: struct.Cnx.html
//! [`QTile`]: http://www.qtile.org/
//! [`dwm`]: http://dwm.suckless.org/
//! [readme-deps]: https://github.com/mjkillough/cnx/blob/master/README.md#dependencies
//! [`src/bin/cnx.rs`]: https://github.com/mjkillough/cnx/blob/master/src/bin/cnx.rs
//! [`Active Window Title`]: widgets/struct.ActiveWindowTitle.html
//! [`EWMH`]: https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html
//! [`Pager`]: widgets/struct.Pager.html
//! [`Sensors`]: widgets/struct.Sensors.html
//! [`lm_sensors`]: https://wiki.archlinux.org/index.php/lm_sensors
//! [`Volume`]: widgets/struct.Volume.html
//! [`Battery`]: widgets/struct.Battery.html
//! [`Clock`]: widgets/struct.Clock.html
//! [`Widget`]: widgets/trait.Widget.html
//! [`widgets`]: widgets/index.html

#![recursion_limit = "256"]

mod bar;
mod cmd;
pub mod text;
pub mod widgets;
mod xcb;

use anyhow::Result;
use tokio::runtime::Runtime;
use tokio::stream::{StreamExt, StreamMap};
use tokio::task;

use crate::bar::Bar;
use crate::widgets::Widget;
use crate::xcb::XcbEventStream;

pub use bar::Position;

/// The main object, used to instantiate an instance of Cnx.
///
/// Widgets can be added using the [`add_widget()`] method. Once configured,
/// the [`run()`] method will take ownership of the instance and run it until
/// the process is killed or an error occurs.
///
/// [`add_widget()`]: #method.add_widget
/// [`run()`]: #method.run
pub struct Cnx {
    position: Position,
    widgets: Vec<Box<dyn Widget>>,
}

impl Cnx {
    /// Creates a new `Cnx` instance.
    ///
    /// This creates a new `Cnx` instance at either the top or bottom of the
    /// screen, depending on the value of the [`Position`] enum.
    ///
    /// [`Position`]: enum.Position.html
    pub fn new(position: Position) -> Self {
        let widgets = Vec::new();
        Self { position, widgets }
    }

    // Adds a widget to the `Cnx` instance.
    //
    // Takes ownership of the [`Widget`] and adds it to the Cnx instance to
    // the right of any existing widgets.
    //
    // [`Widget`]: widgets/trait.Widget.html
    pub fn add_widget<W>(&mut self, widget: W)
    where
        W: Widget + 'static,
    {
        self.widgets.push(Box::new(widget));
    }

    /// Runs the Cnx instance.
    ///
    /// This method takes ownership of the Cnx instance and runs it until either
    /// the process is terminated, or an internal error is returned.
    pub fn run(self) -> Result<()> {
        // Use a single-threaded event loop. We aren't interested in
        // performance too much, so don't mind if we block the loop
        // occasionally. We are using events to get woken up as
        // infrequently as possible (to save battery).
        let mut rt = Runtime::new()?;
        let local = task::LocalSet::new();
        local.block_on(&mut rt, self.run_inner())?;
        Ok(())
    }

    async fn run_inner(self) -> Result<()> {
        let mut bar = Bar::new(self.position)?;

        let mut widgets = StreamMap::with_capacity(self.widgets.len());
        for widget in self.widgets {
            let idx = bar.add_content(Vec::new())?;
            widgets.insert(idx, widget.into_stream()?);
        }

        let mut event_stream = XcbEventStream::new(bar.connection().clone())?;
        task::spawn_local(async move {
            loop {
                tokio::select! {
                    // Pass each XCB event to the Bar.
                    Some(event) = event_stream.next() => {
                        if let Err(err) = bar.process_event(event) {
                            println!("Error processing XCB event: {}", err);
                        }
                    },

                    // Each time a widget yields new values, pass to the bar.
                    // Ignore (but log) any errors from widgets.
                    Some((idx, result)) = widgets.next() => {
                        match result {
                            Err(err) => println!("Error from widget {}: {}", idx, err),
                            Ok(texts) => {
                                if let Err(err) = bar.update_content(idx, texts) {
                                    println!("Error updating widget {}: {}", idx, err);
                                }
                            }
                        }
                    }
                }
            }
        })
        .await?;

        Ok(())
    }
}
