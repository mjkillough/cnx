use anyhow::{Context, Result};
use futures::stream::StreamExt;
use xcb_util::ewmh;

use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};
use crate::xcb::xcb_properties_stream;

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
    attr: Attributes,
}

impl ActiveWindowTitle {
    /// Creates a new Active Window Title widget.
    pub fn new(attr: Attributes) -> ActiveWindowTitle {
        ActiveWindowTitle { attr }
    }

    fn on_change(&self, conn: &ewmh::Connection, screen_idx: i32) -> Vec<Text> {
        let title = ewmh::get_active_window(conn, screen_idx)
            .get_reply()
            .and_then(|active_window| {
                // xcb_properties_stream() will only register for notifications on the
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

        vec![Text {
            attr: self.attr.clone(),
            text: title,
            stretch: true,
            markup: false,
        }]
    }
}

impl Widget for ActiveWindowTitle {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let properties = &["_NET_ACTIVE_WINDOW", "_NET_WM_NAME"];
        let screen_idx = 0; // XXX assume
        let (conn, stream) =
            xcb_properties_stream(properties).context("Initialising ActiveWindowtitle")?;

        let stream = stream.map(move |()| Ok(self.on_change(&conn, screen_idx)));

        Ok(Box::pin(stream))
    }
}
