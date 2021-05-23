use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};
use anyhow::Result;
use async_stream::try_stream;
use std::time::Duration;
use weathernoaa::weather::*;

pub struct Weather {
    attr: Attributes,
    station_code: String,
    render: Option<Box<dyn Fn(WeatherInfo) -> String>>,
}

impl Weather {
    pub fn new(
        attr: Attributes,
        station_code: String,
        render: Option<Box<dyn Fn(WeatherInfo) -> String>>,
    ) -> Weather {
        Weather {
            attr,
            station_code,
            render,
        }
    }
}

impl Widget for Weather {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let stream = try_stream! {
            loop {
                let weather = get_weather(self.station_code.clone()).await?;
                let text = self.render.as_ref().map_or(format!("Temp: {}°C", weather.temperature.celsius), |x| (x)(weather));
                let texts = vec![Text {
                    attr: self.attr.clone(),
                    text,
                    stretch: false,
                    markup: true,
                }];
                yield texts;

                let thirty_minutes = 30 * 60;
                let sleep_for = Duration::from_secs(thirty_minutes);
                tokio::time::sleep(sleep_for).await;
            }
        };
        Ok(Box::pin(stream))
    }
}
