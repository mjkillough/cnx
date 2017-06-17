use std::rc::Rc;
use std::time::Duration;

use chrono::prelude::*;
use tokio_timer::Timer;

use errors::*;
use text::{Attributes, Text};


pub struct Clock {
    timer: Rc<Timer>,
    update_interval: Duration,
    attr: Attributes,
}

impl Clock {
    pub fn new(timer: Rc<Timer>, attr: Attributes) -> Clock {
        Clock {
            timer,
            update_interval: Duration::from_secs(1),
            attr,
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

timer_widget!(Clock, timer, update_interval, tick);
