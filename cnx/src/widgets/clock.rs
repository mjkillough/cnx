use anyhow::Result;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};

/// Shows the current time and date.
///
/// This widget shows the current time and date, in the form `%Y-%m-%d %a %I:%M
/// %p`, e.g. `2017-09-01 Fri 12:51 PM`.
/// modified to '%a %m-%d-%Y %I:%M %p' eg 'Fri 09-01-2017 12:51 PM'
pub struct Clock {
    attr: Attributes,
    format_str: Option<String>,
}

impl Clock {
    // Creates a new Clock widget.
    pub fn new(attr: Attributes, format_str: Option<String>) -> Self {
        Self { attr, format_str }
    }

    fn tick(&self) -> Vec<Text> {
        let now = chrono::Local::now();
        let format_time: String = self
            .format_str
            .clone()
            .map_or("%a %m-%d-%Y %I:%M %p".to_string(), |item| item);
        let text = now.format(&format_time).to_string();
        let texts = vec![Text {
            attr: self.attr.clone(),
            text,
            stretch: false,
            markup: true,
        }];
        texts
    }
}

impl Widget for Clock {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        // As we're not showing seconds, we can sleep for however long
        // it takes until the minutes changes between updates.
        let one_minute = Duration::from_secs(60);
        let interval = time::interval(one_minute);
        let stream = IntervalStream::new(interval).map(move |_| Ok(self.tick()));

        Ok(Box::pin(stream))
    }
}
