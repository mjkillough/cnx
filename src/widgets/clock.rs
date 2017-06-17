use std::time::Duration;

use chrono::prelude::*;

use errors::*;
use text::{Attributes, Text};


pub struct Clock {
    update_interval: Duration,
    attr: Attributes,
}

impl Clock {
    pub fn new(attr: Attributes) -> Clock {
        Clock {
            update_interval: Duration::from_secs(1),
            attr: attr,
        }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let current_time = Local::now().format("%Y-%m-%d %a %I:%M %p").to_string();
        Ok(vec![
            Text {
                attr: self.attr.clone(),
                text: current_time,
                stretch: false,
            },
        ])
    }
}

timer_widget!(Clock, update_interval, tick);
