use crate::text::Color;
use crate::text::{Attributes, Text, Threshold};
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
    threshold: Option<Threshold>,
    template: Option<String>,
}

impl Wireless {
    pub fn new(attr: Attributes, interface: String, threshold: Option<Threshold>) -> Wireless {
        Wireless {
            update_interval: Duration::from_secs(3600),
            interface,
            attr,
            threshold,
            template: None,
        }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let wireless_info = get_wireless_info(self.interface.clone());

        let text = match wireless_info {
            Some(info) => match &self.threshold {
                Some(thold) => {
                    let color = if info.wi_quality <= thold.low.threshold {
                        &thold.low.color
                    } else if info.wi_quality <= thold.normal.threshold {
                        &thold.normal.color
                    } else {
                        &thold.high.color
                    };
                    format!(
                        "<span foreground=\"#808080\">[</span>{} <span foreground=\"{}\">{}%</span><span foreground=\"#808080\">]</span>",
                        info.wi_essid,
                        color.to_hex(),
                        info.wi_quality
                    )
                }
                None => format!("{} {}%", info.wi_essid, info.wi_quality),
            },
            None => "NA".to_owned(),
        };
        Ok(vec![Text {
            attr: self.attr.clone(),
            text,
            stretch: false,
            markup: self.threshold.is_some(),
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
