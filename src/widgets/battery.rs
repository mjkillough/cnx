use std::f64;
use std::fs::File;
use std::io::Read;
use std::result;
use std::str::FromStr;
use std::time::Duration;

use failure::{format_err, Error, ResultExt};
use tokio_timer::Timer;

use crate::text::{Attributes, Color, Text};
use crate::{Cnx, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Status {
    Full,
    Charging,
    Discharging,
    Unknown,
}

impl FromStr for Status {
    type Err = Error;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "Full" => Ok(Status::Full),
            "Charging" => Ok(Status::Charging),
            "Discharging" => Ok(Status::Discharging),
            "Unknown" => Ok(Status::Unknown),
            _ => Err(format_err!("Unknown Status: {}", s)),
        }
    }
}

/// Shows battery charge percentage and (dis)charge time.
///
/// This widget shows the battery's current charge percentage and the amount of
/// remaining (dis)charge time, depending on whether the battery is charging or
/// discharging. The format of the output is `(PP% HH:MM)`.
///
/// When the battery has less than 10% charge remaining, the widget's text will
/// change to the specified `warning_color`.
///
/// Battery charge information is read from [`/sys/class/power_supply/BAT0/`].
///
/// [`/sys/class/power_supply/BAT0/`]: https://www.kernel.org/doc/Documentation/power/power_supply_class.txt
pub struct Battery {
    timer: Timer,
    update_interval: Duration,
    battery: String,
    attr: Attributes,
    warning_color: Color,
}

impl Battery {
    ///  Creates a new Battery widget.
    ///
    ///  Creates a new `Battery` widget, whose text will be displayed with the
    ///  given [`Attributes`]. The caller can provide use the `warning_color`
    ///  argument, to control the [`Color`] of the text once the battery has
    ///  less than 10% charge remaining.
    ///
    ///  The [`Cnx`] instance is borrowed during construction in order to get
    ///  access to handles of its event loop. However, it is not borrowed for
    ///  the lifetime of the widget. See the [`cnx_add_widget!()`] for more
    ///  discussion about the lifetime of the borrow.
    ///
    /// [`Attributes`]: ../text/struct.Attributes.html
    /// [`Color`]: ../text/struct.Color.html
    /// [`Cnx`]: ../struct.Cnx.html
    /// [`cnx_add_widget!()`]: ../macro.cnx_add_widget.html
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
    /// #
    /// # fn run() -> ::cnx::Result<()> {
    /// let attr = Attributes {
    ///     font: Font::new("SourceCodePro 21"),
    ///     fg_color: Color::white(),
    ///     bg_color: None,
    ///     padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    /// };
    ///
    /// let mut cnx = Cnx::new(Position::Top)?;
    /// cnx_add_widget!(cnx, Battery::new(&cnx, attr.clone(), Color::red()));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(cnx: &Cnx, attr: Attributes, warning_color: Color) -> Battery {
        Battery {
            timer: cnx.timer(),
            update_interval: Duration::from_secs(60),
            battery: "BAT0".to_owned(),
            attr,
            warning_color,
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
            .with_context(|_| format!("Could not load value from battery status file: {}", file))?;
        Ok(value)
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let full: f64 = self.load_value("charge_full")?;
        let now: f64 = self.load_value("charge_now")?;
        let percentage = (now / full) * 100.0;

        // If we're discharging, show time to empty.
        // If we're charging, show time to full.
        let power: f64 = self
            .load_value("current_avg")
            .or_else(|_| self.load_value("current_now"))?;
        let status: Status = self.load_value("status")?;
        let time = match status {
            Status::Discharging => now / power,
            Status::Charging => (full - now) / power,
            _ => 0.0,
        };
        let hours = time as u64;
        let minutes = (time * 60.0) as u64 % 60;

        let text = format!(
            "({percentage:.0}% - {hours}:{minutes:02})",
            percentage = percentage,
            hours = hours,
            minutes = minutes
        );

        // If we're discharging and have <=10% left, then render with a
        // special warning color.
        let mut attr = self.attr.clone();
        if status == Status::Discharging && percentage <= 10.0 {
            attr.fg_color = self.warning_color.clone()
        }

        Ok(vec![Text {
            attr,
            text,
            stretch: false,
        }])
    }
}

timer_widget!(Battery, timer, update_interval, tick);
