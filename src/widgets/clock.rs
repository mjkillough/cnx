use anyhow::Result;
use async_stream::try_stream;
use chrono::Timelike;
use std::time::Duration;

use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};

/// Shows the current time and date.
///
/// This widget shows the current time and date, in the form `%Y-%m-%d %a %I:%M
/// %p`, e.g. `2017-09-01 Fri 12:51 PM`.
pub struct Clock {
    attr: Attributes,
}

impl Clock {
    // Creates a new Clock widget.
    pub fn new(attr: Attributes) -> Self {
        Self { attr }
    }
}

impl Widget for Clock {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let stream = try_stream! {
            loop {
                let now = chrono::Local::now();
                let text = now.format("<span foreground=\"#808080\">[</span>%d-%m-%Y %a %I:%M %p<span foreground=\"#808080\">]</span>").to_string();
                let texts = vec![Text {
                    attr: self.attr.clone(),
                    text,
                    stretch: false,
                    markup: true
                }];

                yield texts;

                // As we're not showing seconds, we can sleep for however long
                // it takes until the minutes changes between updates.
                let sleep_for = Duration::from_secs(60 - u64::from(now.second()));
                tokio::time::sleep(sleep_for).await;
            }
        };

        Ok(Box::pin(stream))
    }
}
