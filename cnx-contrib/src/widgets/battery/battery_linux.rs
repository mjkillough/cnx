use anyhow::{anyhow, Context, Error, Result};
use cnx::text::{Attributes, Color, Text};
use cnx::widgets::{Widget, WidgetStream};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

/// Represent Battery's operating status
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    Full,
    Charging,
    Discharging,
    Unknown,
}

impl FromStr for Status {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Full" => Ok(Status::Full),
            "Charging" => Ok(Status::Charging),
            "Discharging" => Ok(Status::Discharging),
            "Unknown" => Ok(Status::Unknown),
            _ => Err(anyhow!("Unknown Status: {}", s)),
        }
    }
}

/// Shows battery charge percentage
///
/// This widget shows the battery's current charge percentage.
///
/// When the battery has less than 10% charge remaining, the widget's text will
/// change to the specified `warning_color`.
///
/// Battery charge information is read from [`/sys/class/power_supply/BAT0/`].
///
/// [`/sys/class/power_supply/BAT0/`]: https://www.kernel.org/doc/Documentation/power/power_supply_class.txt
pub struct Battery {
    update_interval: Duration,
    battery: String,
    attr: Attributes,
    warning_color: Color,
    render: Option<Box<dyn Fn(BatteryInfo) -> String>>,
}

/// Represent Battery information
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatteryInfo {
    /// Battery Status
    pub status: Status,
    /// Capacity in percentage
    pub capacity: u8,
}

impl Battery {
    ///  Creates a new Battery widget.
    ///
    ///  Creates a new `Battery` widget, whose text will be displayed with the
    ///  given [`Attributes`]. The caller can provide use the `warning_color`
    ///  argument, to control the [`Color`] of the text once the battery has
    ///  less than 10% charge remaining.
    ///
    ///  The [`cnx::Cnx`] instance is borrowed during construction in order to get
    ///  access to handles of its event loop. However, it is not borrowed for
    ///  the lifetime of the widget. See the [`cnx::Cnx::add_widget`] for more
    ///  discussion about the lifetime of the borrow.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate cnx;
    /// #
    /// # use cnx::*;
    /// # use cnx::text::*;
    /// # use cnx::widgets::*;
    /// # use cnx_contrib::widgets::battery::*;
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
    /// cnx.add_widget(Battery::new(attr.clone(), Color::red(), None, None));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(
        attr: Attributes,
        warning_color: Color,
        battery: Option<String>,
        render: Option<Box<dyn Fn(BatteryInfo) -> String>>,
    ) -> Battery {
        Battery {
            update_interval: Duration::from_secs(60),
            battery: battery.unwrap_or_else(|| "BAT0".into()),
            attr,
            warning_color,
            render,
        }
    }

    fn load_value_inner<T>(&self, file: &str) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: Into<Error>,
    {
        let path = format!("/sys/class/power_supply/{}/{}", self.battery, file);
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let s = FromStr::from_str(contents.trim())
            .map_err(|e: <T as FromStr>::Err| e.into())
            .context("Failed to parse value")?;
        Ok(s)
    }

    fn load_value<T>(&self, file: &str) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: Into<Error>,
    {
        let value = self
            .load_value_inner(file)
            .with_context(|| format!("Could not load value from battery status file: {file}"))?;
        Ok(value)
    }

    fn get_value(&self) -> Result<BatteryInfo> {
        let capacity: u8 = self.load_value("capacity")?;
        let status: Status = self.load_value("status")?;
        Ok(BatteryInfo { capacity, status })
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let battery_info = self.get_value()?;

        let default_text = format!("({percentage:.0}%)", percentage = battery_info.capacity,);
        let text = self
            .render
            .as_ref()
            .map_or(default_text, |x| (x)(battery_info.clone()));

        // If we're discharging and have <=10% left, then render with a
        // special warning color.
        let mut attr = self.attr.clone();
        if battery_info.status == Status::Discharging && battery_info.capacity <= 10 {
            attr.fg_color = self.warning_color.clone()
        }

        Ok(vec![Text {
            attr,
            text,
            stretch: false,
            markup: self.render.is_some(),
        }])
    }
}

impl Widget for Battery {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let interval = time::interval(self.update_interval);
        let stream = IntervalStream::new(interval).map(move |_| self.tick());

        Ok(Box::pin(stream))
    }
}
