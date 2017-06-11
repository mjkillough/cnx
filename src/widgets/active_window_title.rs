use text::{Attributes, Text};

use xcb;
use xcb_util;


pub struct ActiveWindowTitle {
    conn: xcb_util::ewmh::Connection,
    screen_idx: i32,

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

            attr: attr,
        }
    }

    fn get_active_window_title(&self) -> String {
        let active_window = xcb_util::ewmh::get_active_window(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap();
        let reply = xcb_util::ewmh::get_wm_name(&self.conn, active_window).get_reply();

        match reply {
            Ok(inner) => inner.string().to_owned(),
            // Probably means there's no window focused, or it doesn't have _NET_WM_NAME:
            Err(_) => "".to_owned(),
        }
    }

    pub fn compute_text(&self) -> Vec<Text> {
        vec![Text {
                 attr: self.attr.clone(),
                 text: self.get_active_window_title(),
                 stretch: true,
             }]
    }
}
