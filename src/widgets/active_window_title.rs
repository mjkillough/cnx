use text::{Attributes, Text};

use tokio_core::reactor::Handle;
use xcb;
use xcb_util::ewmh;


pub struct ActiveWindowTitle {
    tokio_handle: Handle,
    attr: Attributes,
}

impl ActiveWindowTitle {
    pub fn new(tokio_handle: Handle, attr: Attributes) -> ActiveWindowTitle {
        ActiveWindowTitle { tokio_handle, attr }
    }

    fn on_change(&self, conn: &ewmh::Connection, screen_idx: i32) -> Vec<Text> {
        let active_window = ewmh::get_active_window(conn, screen_idx)
            .get_reply()
            .unwrap();
        let reply = ewmh::get_wm_name(conn, active_window).get_reply();

        // x_properties_widget!() will only register for notifications on the
        // root window, so will only receive notifications when the active window
        // changes. So, for each active window we see, register for property
        // change notifications, so that we can see when the currently active
        // window changes title. (We'll continue to receive notifications after
        // it is no longer the active window, but this isn't a big deal).
        let attributes = [(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_PROPERTY_CHANGE)];
        xcb::change_window_attributes(conn, active_window, &attributes);
        conn.flush();

        let title = match reply {
            Ok(inner) => inner.string().to_owned(),
            // Probably means there's no window focused, or it doesn't have _NET_WM_NAME:
            Err(_) => "".to_owned(),
        };

        vec![Text {
                 attr: self.attr.clone(),
                 text: title,
                 stretch: true,
             }]
    }
}

x_properties_widget!(ActiveWindowTitle, tokio_handle, on_change; [
    ACTIVE_WINDOW,
    WM_NAME
]);
