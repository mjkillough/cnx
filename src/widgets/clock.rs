use std::time::Duration;

use chrono::prelude::*;
use futures::{stream, Future, Stream};
use tokio_timer::Timer;

use Cnx;
use errors::*;
use text::{Attributes, Text};
use super::{Widget, WidgetStream};

/// Shows the current time and date.
///
/// This widget shows the current time and date, in the form `%Y-%m-%d %a %I:%M
/// %p`, e.g. `2017-09-01 Fri 12:51 PM`.
pub struct Clock {
    timer: Timer,
    attr: Attributes,
}

impl Clock {
    /// Creates a new Clock widget.
    ///
    /// Creates a new `Clock` widget, whose text will be displayed with the
    /// given [`Attributes`].
    ///
    /// The [`Cnx`] instance is borrowed during construction in order to get
    /// access to handles of its event loop. However, it is not borrowed for the
    /// lifetime of the widget. See the [`cnx_add_widget!()`] for more
    /// discussion about the lifetime of the borrow.
    ///
    /// [`Attributes`]: ../text/struct.Attributes.html
    /// [`Cnx`]: ../struct.Cnx.html
    /// [`cnx_add_widget!()`]: ../macro.cnx_add_widget.html
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate cnx;
    /// #
    /// # use cnx::*;
    /// # use cnx::text::*;
    /// # use cnx::widgets::*;
    /// #
    /// # fn run() -> ::cnx::errors::Result<()> {
    /// let attr = Attributes {
    ///     font: Font::new("SourceCodePro 21"),
    ///     fg_color: Color::white(),
    ///     bg_color: None,
    ///     padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    /// };
    ///
    /// let mut cnx = Cnx::new(Position::Top)?;
    /// cnx_add_widget!(cnx, Clock::new(&cnx, attr.clone()));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(cnx: &Cnx, attr: Attributes) -> Clock {
        Clock {
            timer: cnx.timer(),
            attr,
        }
    }
}

impl Widget for Clock {
    fn stream(self: Box<Self>) -> Result<WidgetStream> {
        // As we're not showing seconds, we can sleep for however long it takes
        // until the minutes changes between updates. Initially sleep for 0 seconds
        // so that our `self.timer.sleep()` expires immediately.
        let sleep_for = Duration::from_secs(0);
        let stream = stream::unfold(sleep_for, move |sleep_for| {
            // Avoid having to move self into the .map() closure.
            let attr = self.attr.clone();
            Some(self.timer.sleep(sleep_for).map(move |()| {
                let now = Local::now();
                let formatted = now.format("%Y-%m-%d %a %I:%M %p").to_string();
                let texts = vec![
                    Text {
                        attr: attr,
                        text: formatted,
                        stretch: false,
                    },
                ];

                let sleep_for = Duration::from_secs(60 - now.second() as u64);
                (texts, sleep_for)
            }))
        }).then(|r| r.chain_err(|| "Error in tokio_timer stream"));

        Ok(Box::new(stream))
    }
}
