use super::TimerUpdateWidget;
use text::{Attributes, Text};

use std::time::Duration;

use chrono::prelude::*;
use xcb;
use xcb_util;


pub struct Clock {
    conn: xcb::Connection,
    update_interval: Duration,
    attr: Attributes,
}

impl Clock {
    pub fn new(attr: Attributes) -> Clock {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();

        Clock {
            conn: conn,
            update_interval: Duration::from_secs(1),
            attr: attr,
        }
    }
}

impl TimerUpdateWidget for Clock {
    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    fn tick(&self) -> Vec<Text> {
        let current_time = Local::now().format("%Y-%m-%d %a %I:%M %p").to_string();
        vec![Text {
                 attr: self.attr.clone(),
                 text: current_time,
                 stretch: false,
             }]
    }
}
