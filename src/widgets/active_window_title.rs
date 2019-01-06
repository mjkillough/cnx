use tokio_core::reactor::Handle;
use xcb;
use xcb_util::ewmh;

use errors::*;
use text::{Attributes, Text};
use Cnx;

/// Shows the title of the currently focused window.
///
/// This widget shows the title (`_NET_WM_NAME` [`EWMH`] property) of the
/// currently focused window. It uses the `_NET_ACTIVE_WINDOW` [`EWMH`] property
/// of the root window to determine which window is currently focused.
///
/// The widgets content stretches to fill all available space. If the title is
/// too large for the available space, it will be truncated.
///
/// [`EWMH`]: https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html
pub struct ActiveWindowTitle {
    tokio_handle: Handle,
    attr: Attributes,
}

impl ActiveWindowTitle {
    /// Creates a new Active Window Title widget.
    ///
    /// Creates a new `ActiveWindowTitle` widget, whose text will be displayed
    /// with the given [`Attributes`].
    ///
    /// The [`Cnx`] instance is borrowed during construction in order to get
    /// access to handles of its event loop. However, it is not borrowed for the
    /// lifetime of the widget. See the [`cnx_add_widget!()`] for more discussion
    /// about the lifetime of the borrow.
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
    /// cnx_add_widget!(cnx, ActiveWindowTitle::new(&cnx, attr.clone()));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(cnx: &Cnx, attr: Attributes) -> ActiveWindowTitle {
        ActiveWindowTitle {
            tokio_handle: cnx.handle(),
            attr,
        }
    }

    fn on_change(&self, conn: &ewmh::Connection, screen_idx: i32) -> Result<Vec<Text>> {
        let title = ewmh::get_active_window(conn, screen_idx)
            .get_reply()
            .and_then(|active_window| {
                // x_properties_widget!() will only register for notifications on the
                // root window, so will only receive notifications when the active window
                // changes. So, for each active window we see, register for property
                // change notifications, so that we can see when the currently active
                // window changes title. (We'll continue to receive notifications after
                // it is no longer the active window, but this isn't a big deal).
                let attributes = [(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_PROPERTY_CHANGE)];
                xcb::change_window_attributes(conn, active_window, &attributes);
                conn.flush();

                ewmh::get_wm_name(conn, active_window).get_reply()
            })
            .map(|reply| reply.string().to_owned())
            .unwrap_or_else(|_| "".to_owned());

        Ok(vec![Text {
            attr: self.attr.clone(),
            text: title,
            stretch: true,
        }])
    }
}

x_properties_widget!(ActiveWindowTitle, tokio_handle, on_change; [
    ACTIVE_WINDOW,
    WM_NAME
]);
