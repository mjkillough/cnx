use std::time::Duration;

use chrono::prelude::*;
use futures::{stream, Future, Stream};
use tokio_timer::Timer;

use Cnx;
use errors::*;
use text::{Attributes, Text};
use super::{Widget, WidgetStream};


pub struct Clock {
    timer: Timer,
    attr: Attributes,
}

impl Clock {
    pub fn new(cnx: &Cnx, attr: Attributes) -> Clock {
        Clock {
            timer: cnx.timer(),
            attr,
        }
    }
}

impl Widget for Clock {
    fn stream(self: Box<Self>) -> Result<WidgetStream> {
        // As we're not showing seconds, we can sleep for however long it takes
        // until the minutes changes between updates. Initially sleep for 0 seconds
        // so that our `self.timer.sleep()` expires immediately.
        let sleep_for = Duration::from_secs(0);
        let stream = stream::unfold(sleep_for, move |sleep_for| {
            // Avoid having to move self into the .map() closure.
            let attr = self.attr.clone();
            Some(self.timer.sleep(sleep_for)
                .map(move |()| {
                    let now = Local::now();
                    let formatted = now.format("%Y-%m-%d %a %I:%M %p").to_string();
                    let texts = vec![
                        Text {
                            attr: attr,
                            text: formatted,
                            stretch: false,
                        },
                    ];

                    let sleep_for = Duration::from_secs(60 - now.second() as u64);
                    (texts, sleep_for)
                }))
        }).then(|r| r.chain_err(|| "Error in tokio_timer stream"));

        Ok(Box::new(stream))
    }
}
