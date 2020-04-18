use std::str::FromStr;
use std::time::Duration;

use failure::ResultExt;
use tokio_timer::Timer;

use crate::cmd::{command_output, from_command_output};
use crate::text::{Attributes, Color, Text};
use crate::{Cnx, Result};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Status {
    Charging,
    Discharging,
    Unknown,
}

#[derive(Copy, Clone, Debug)]
struct Info {
    status: Status,
    minutes: Option<u16>,
    percentage: u8, // 0-100
}

impl Info {
    fn time_remaining(&self) -> Option<(u16, u16)> {
        if let Some(minutes) = self.minutes {
            let hours = minutes / 60;
            let minutes = minutes % 60;
            return Some((hours, minutes));
        }
        None
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
            attr,
            warning_color,
        }
    }

    fn load_status(&self) -> Result<Status> {
        let string = command_output("apm", &["-a"])?;
        match string.trim() {
            "0" => Ok(Status::Discharging),
            "1" => Ok(Status::Charging),
            _ => Ok(Status::Unknown),
        }
    }

    fn load_percentage(&self) -> Result<u8> {
        let percentage = from_command_output("apm", &["-l"]).context("Battery percentage")?;
        Ok(percentage)
    }

    fn load_time_remaining(&self) -> Result<Option<u16>> {
        let string = command_output("apm", &["-m"])?;
        if string.trim() == "unknown" {
            return Ok(None);
        }
        let minutes = u16::from_str(string.trim()).context("Parsing time remaining")?;
        Ok(Some(minutes))
    }

    fn load_info(&self) -> Result<Info> {
        let status = self.load_status()?;
        let percentage = self.load_percentage()?;
        let minutes = self.load_time_remaining()?;
        Ok(Info {
            status,
            percentage,
            minutes,
        })
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let info = self.load_info()?;

        let mut text = match info.status {
            Status::Charging => "(⚡ ".to_owned(),
            _ => "(".to_owned(),
        };
        text += &format!("{:.0}%", info.percentage);
        if let Some((hours, minutes)) = info.time_remaining() {
            text += &format!(" - {hours}:{minutes:02})", hours = hours, minutes = minutes);
        } else {
            text += ")";
        }

        // If we're discharging and have <=10% left, then render with a
        // special warning color.
        let mut attr = self.attr.clone();
        if info.status == Status::Discharging && info.percentage <= 10 {
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
