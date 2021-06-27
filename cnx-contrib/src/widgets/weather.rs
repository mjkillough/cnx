use anyhow::Result;
use async_stream::try_stream;
use cnx::text::{Attributes, Text};
use cnx::widgets::{Widget, WidgetStream};
use std::time::Duration;
use weathernoaa::weather::*;

/// Represents Weather widget used to show current weather information.
pub struct Weather {
    attr: Attributes,
    station_code: String,
    render: Option<Box<dyn Fn(WeatherInfo) -> String>>,
}

impl Weather {
    /// Creates a new [`Weather`] widget.
    ///
    /// Arguments
    ///
    /// * `attr` - Represents `Attributes` which controls properties like
    /// `Font`, foreground and background color etc.
    ///
    /// * `station_code` - Represents weather station code from the
    /// Federal Climate Complex ISD. You can find your place's station
    /// code by getting the information from either [NOAA's
    /// archive](https://www1.ncdc.noaa.gov/pub/data/noaa/isd-history.txt)
    /// or [Internet Archive's
    /// data](https://web.archive.org/web/20210522235412/https://www1.ncdc.noaa.gov/pub/data/noaa/isd-history.txt)
    /// of the same link.
    ///
    /// * `render` - We use the closure to control the way output is
    /// displayed in the bar. [`WeatherInfo`] represents the current
    /// weather details of the particular station.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate cnx;
    /// #
    /// # use cnx::*;
    /// # use cnx::text::*;
    /// # use cnx_contrib::widgets::weather::*;
    /// # use anyhow::Result;
    /// #
    /// # fn run() -> Result<()> {
    /// let attr = Attributes {
    ///     font: Font::new("SourceCodePro 21"),
    ///     fg_color: Color::white(),
    ///     bg_color: None,
    ///     padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    /// };
    ///
    /// let mut cnx = Cnx::new(Position::Top);
    /// cnx.add_widget(Weather::new(attr, "VOBL".into(),  None));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
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
