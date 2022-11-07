use anyhow::{Context, Result};
use futures::stream::StreamExt;
use std::cmp::Ordering;
use xcb_util::ewmh;

use crate::text::{Attributes, PagerAttributes, Text};
use crate::widgets::{Widget, WidgetStream};
use crate::xcb::xcb_properties_stream;

/// Shows the WM's workspaces/groups.
///
/// This widget can highlight the currently active, inactive and non empty
/// workspaces/groups differently.
///
/// This widget shows the WM's workspaces/groups, as determined by the [`EWMH`]
/// `_NET_NUMBER_OF_DESKTOPS`, `_NET_DESKTOP_NAMES`, `_NET_CURRENT_DESKTOP`,
/// `_NET_CLIENT_LIST`, `_NET_WM_WINDOW_TYPE` and `_NET_WM_DESKTOP` properties.
///
/// [`EWMH`]: https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html
pub struct Pager {
    active_attr: Attributes,
    inactive_attr: Attributes,
    non_empty_attr: Attributes,
}

impl Pager {
    ///  Creates a new Pager widget.
    pub fn new(pager_attrs: PagerAttributes) -> Self {
        Self {
            active_attr: pager_attrs.active_attr,
            inactive_attr: pager_attrs.inactive_attr,
            non_empty_attr: pager_attrs.non_empty_attr,
        }
    }

    fn on_change(&self, conn: &ewmh::Connection, screen_idx: i32) -> Vec<Text> {
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
        match names.len().cmp(&number) {
            Ordering::Equal => (),
            Ordering::Greater => names.truncate(number),
            Ordering::Less => {
                let num_unnamed = number - names.len();
                names.extend(vec!["?"; num_unnamed]);
            }
        }

        names
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                let attr = if i == current {
                    self.active_attr.clone()
                } else if non_empty_desktops(conn, screen_idx).contains(&(i as u32)) {
                    self.non_empty_attr.clone()
                } else {
                    self.inactive_attr.clone()
                };

                Text {
                    attr,
                    text: name.to_owned(),
                    stretch: false,
                    markup: true,
                }
            })
            .collect()
    }
}

fn non_empty_desktops(conn: &ewmh::Connection, screen_idx: i32) -> Vec<u32> {
    let client_list = ewmh::get_client_list(conn, screen_idx).get_reply();
    let windows: &[u32] = match client_list {
        Ok(ref cl) => cl.windows(),
        Err(_) => &[],
    };

    windows
        .iter()
        .filter(|&w| match ewmh::get_wm_window_type(conn, *w).get_reply() {
            Ok(wt) => wt.atoms().first() == Some(&conn.WM_WINDOW_TYPE_NORMAL()),
            Err(_) => false,
        })
        .filter_map(|w| ewmh::get_wm_desktop(conn, *w).get_reply().ok())
        .filter(
            |&d| match ewmh::get_current_desktop(conn, screen_idx).get_reply() {
                Ok(c) => c != d,
                Err(_) => true,
            },
        )
        .collect()
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

        let stream = stream.map(move |()| Ok(self.on_change(&conn, screen_idx)));

        Ok(Box::pin(stream))
    }
}
