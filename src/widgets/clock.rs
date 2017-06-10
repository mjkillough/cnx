use text::{Attributes, Text};

use chrono::prelude::*;

use xcb;
use xcb_util;


pub struct Clock {
    conn: xcb::Connection,

    attr: Attributes,
}

impl Clock {
    pub fn new(attr: Attributes) -> Clock {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();

        Clock {
            conn: conn,
            attr: attr,
        }
    }

    pub fn compute_text(&self) -> Vec<Text> {
        let current_time = Local::now().format("%Y-%m-%d %a %I:%M %p").to_string();
        vec![Text {
                 attr: self.attr.clone(),
                 text: current_time,
                 stretch: false,
             }]
    }
}
