use text::{Attributes, Text};

use xcb;
use xcb_util;


pub struct Pager {
    conn: xcb_util::ewmh::Connection,
    screen_idx: i32,

    active_attr: Attributes,
    inactive_attr: Attributes,
}

impl Pager {
    pub fn new(active_attr: Attributes, inactive_attr: Attributes) -> Pager {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
        let ewmh_conn = xcb_util::ewmh::Connection::connect(conn)
            .map_err(|_| ())
            .unwrap();

        Pager {
            conn: ewmh_conn,
            screen_idx: screen_idx,
            active_attr: active_attr,
            inactive_attr: inactive_attr,
        }
    }

    fn get_desktops_info(&self) -> Vec<(bool, String)> {
        let number = xcb_util::ewmh::get_number_of_desktops(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap() as usize;
        let current = xcb_util::ewmh::get_current_desktop(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap() as usize;
        let names_reply = xcb_util::ewmh::get_desktop_names(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap();
        let mut names = names_reply.strings();

        // EWMH states that `number` may not equal `names.len()`, as there may
        // be unnamed desktops, or more desktops than are currently in use.
        if names.len() > number {
            names.truncate(number);
        } else if number > names.len() {
            for i in 0..(number - names.len()) {
                names.push("?");
            }
        }

        names
            .into_iter()
            .enumerate()
            .map(|(i, name)| (i == current, name.to_owned()))
            .collect()
    }

    pub fn compute_text(&self) -> Vec<Text> {
        let desktops = self.get_desktops_info();
        self.get_desktops_info()
            .into_iter()
            .map(|(active, name)| {
                Text {
                    attr: if active {
                        self.active_attr.clone()
                    } else {
                        self.inactive_attr.clone()
                    },
                    text: name,
                    stretch: false,
                }
            })
            .collect()
    }
}
