use crate::text::{Attributes, Color, Text};
use crate::widgets::{Widget, WidgetStream};
use anyhow::{anyhow, Context, Error, Result};
use iwlib::*;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

pub struct Wireless {
    attr: Attributes,
    interface: String,
    update_interval: Duration,
}

impl Wireless {
    pub fn new(attr: Attributes, interface: String) -> Wireless {
        Wireless {
            update_interval: Duration::from_secs(3600),
            interface,
            attr,
        }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let wireless_info = get_wireless_info(self.interface.clone());
        let text = match wireless_info {
            Some(info) => format!("{} {}", info.wi_essid, info.wi_quality),
            None => "NA".to_owned(),
        };
        Ok(vec![Text {
            attr: self.attr.clone(),
            text,
            stretch: false,
        }])
    }
}

impl Widget for Wireless {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let interval = time::interval(self.update_interval);
        let stream = IntervalStream::new(interval).map(move |_| self.tick());

        Ok(Box::pin(stream))
    }
}
