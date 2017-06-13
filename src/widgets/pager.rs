use text::{Attributes, Text};

use std::time::Duration;

use xcb;
use xcb_util;


pub struct Pager {
    conn: xcb_util::ewmh::Connection,
    screen_idx: i32,
    update_interval: Duration,
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
            update_interval: Duration::from_secs(1),
            active_attr: active_attr,
            inactive_attr: inactive_attr,
        }
    }

    fn tick(&self) -> Vec<Text> {
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
            let num_unnamed = number - names.len();
            names.extend(vec!["?"; num_unnamed]);
        }

        names
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
            .collect()
    }
}

timer_widget!(Pager, update_interval, tick);
