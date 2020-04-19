use anyhow::{Context, Result};
use tokio::stream::StreamExt;
use xcb_util::ewmh;

use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};
use crate::xcb::xcb_properties_stream;

/// Shows the WM's workspaces/groups, highlighting whichever is currently
/// active.
///
/// This widget shows the WM's workspaces/groups, as determined by the [`EWMH`]
/// `_NET_NUMBER_OF_DESKTOPS` and `_NET_DESKTOP_NAMES` and
/// `_NET_CURRENT_DESKTOP` properties. The active workspace is highlighted.
///
/// [`EWMH`]: https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html
pub struct Pager {
    active_attr: Attributes,
    inactive_attr: Attributes,
}

impl Pager {
    ///  Creates a new Pager widget.
    pub fn new(active_attr: Attributes, inactive_attr: Attributes) -> Self {
        Self {
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
                    attr,
                    text: name.to_owned(),
                    stretch: false,
                }
            })
            .collect())
    }
}

impl Widget for Pager {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let properties = &[
            "_NET_NUMBER_OF_DESKTOPS",
            "_NET_CURRENT_DESKTOP",
            "_NET_DESKTOP_NAMES",
        ];
        let screen_idx = 0; // XXX assume
        let (conn, stream) = xcb_properties_stream(properties).context("Initialising Pager")?;

        let stream = stream.map(move |()| self.on_change(&conn, screen_idx));

        Ok(Box::pin(stream))
    }
}
