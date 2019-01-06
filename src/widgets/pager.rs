use tokio_core::reactor::Handle;
use xcb_util::ewmh;

use errors::*;
use text::{Attributes, Text};
use Cnx;

/// Shows the WM's workspaces/groups, highlighting whichever is currently
/// active.
///
/// This widget shows the WM's workspaces/groups, as determined by the [`EWMH`]
/// `_NET_NUMBER_OF_DESKTOPS` and `_NET_DESKTOP_NAMES` and
/// `_NET_CURRENT_DESKTOP` properties. The active workspace is highlighted.
///
/// [`EWMH`]: https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html
pub struct Pager {
    tokio_handle: Handle,
    active_attr: Attributes,
    inactive_attr: Attributes,
}

impl Pager {
    ///  Creates a new Pager widget.
    ///
    ///  Creates a new `Pager` widget. The widget will list the current
    ///  workspaces in order, using the given `inactive_attr` [`Attributes`] for
    ///  all inactive groups, and the `active_attr` [`Attributes`] for the
    ///  currently active group.
    ///
    ///  The [`Cnx`] instance is borrowed during construction in order to get
    ///  access to handles of its event loop. However, it is not borrowed for
    ///  the lifetime of the widget. See the [`cnx_add_widget!()`] for more
    ///  discussion about the lifetime of the borrow.
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
    /// let mut active_attr = attr.clone();
    /// active_attr.bg_color = Some(Color::blue());
    ///
    /// let mut cnx = Cnx::new(Position::Top)?;
    /// cnx_add_widget!(cnx, Pager::new(&cnx, active_attr, attr.clone()));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(cnx: &Cnx, active_attr: Attributes, inactive_attr: Attributes) -> Pager {
        Pager {
            tokio_handle: cnx.handle(),
            active_attr,
            inactive_attr,
        }
    }

    fn on_change(&self, conn: &ewmh::Connection, screen_idx: i32) -> Result<Vec<Text>> {
        let number = ewmh::get_number_of_desktops(conn, screen_idx)
            .get_reply()
            .unwrap_or(0) as usize;
        let current = ewmh::get_current_desktop(conn, screen_idx)
            .get_reply()
            .unwrap_or(0) as usize;
        let names_reply = ewmh::get_desktop_names(conn, screen_idx).get_reply();
        let mut names = match names_reply {
            Ok(ref r) => r.strings(),
            Err(_) => Vec::new(),
        };

        // EWMH states that `number` may not equal `names.len()`, as there may
        // be unnamed desktops, or more desktops than are currently in use.
        if names.len() > number {
            names.truncate(number);
        } else if number > names.len() {
            let num_unnamed = number - names.len();
            names.extend(vec!["?"; num_unnamed]);
        }

        Ok(names
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                let attr = if i == current {
                    self.active_attr.clone()
                } else {
                    self.inactive_attr.clone()
                };
                Text {
                    attr: attr,
                    text: name.to_owned(),
                    stretch: false,
                }
            })
            .collect())
    }
}

x_properties_widget!(Pager, tokio_handle, on_change; [
    NUMBER_OF_DESKTOPS,
    CURRENT_DESKTOP,
    DESKTOP_NAMES
]);
