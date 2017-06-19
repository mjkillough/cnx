use tokio_core::reactor::Handle;
use xcb;
use xcb_util::ewmh;

use Hue;
use errors::*;
use text::{Attributes, Text};


pub struct ActiveWindowTitle {
    tokio_handle: Handle,
    attr: Attributes,
}

impl ActiveWindowTitle {
    pub fn new(hue: &Hue, attr: Attributes) -> ActiveWindowTitle {
        ActiveWindowTitle {
            tokio_handle: hue.handle(),
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

        Ok(vec![
            Text {
                attr: self.attr.clone(),
                text: title,
                stretch: true,
            },
        ])
    }
}

x_properties_widget!(ActiveWindowTitle, tokio_handle, on_change; [
    ACTIVE_WINDOW,
    WM_NAME
]);
