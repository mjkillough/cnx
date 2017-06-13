use text::{Attributes, Text};

use std::time::Duration;

use xcb;
use xcb_util;


pub struct ActiveWindowTitle {
    conn: xcb_util::ewmh::Connection,
    screen_idx: i32,
    update_interval: Duration,
    attr: Attributes,
}

impl ActiveWindowTitle {
    pub fn new(attr: Attributes) -> ActiveWindowTitle {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
        let ewmh_conn = xcb_util::ewmh::Connection::connect(conn)
            .map_err(|_| ())
            .unwrap();

        ActiveWindowTitle {
            conn: ewmh_conn,
            screen_idx: screen_idx,
            update_interval: Duration::from_secs(1),
            attr: attr,
        }
    }

    fn tick(&self) -> Vec<Text> {
        let active_window = xcb_util::ewmh::get_active_window(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap();
        let reply = xcb_util::ewmh::get_wm_name(&self.conn, active_window).get_reply();

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

timer_widget!(ActiveWindowTitle, update_interval, tick);
