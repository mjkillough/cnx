use anyhow::{Context, Result};
use cnx::cmd::{command_output, from_command_output};
use cnx::text::{Attributes, Color, Text};
use cnx::widgets::{Widget, WidgetStream};
use std::str::FromStr;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

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

#[derive(Default)]
struct OpenBsd;

impl OpenBsd {
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
}

// TODO: Gate this on platform once we add the Linux impl back.
type BatteryInfo = OpenBsd;

/// Shows battery charge percentage and (dis)charge time.
///
/// This widget shows the battery's current charge percentage and the amount of
/// remaining (dis)charge time, depending on whether the battery is charging or
/// discharging. The format of the output is `(PP% HH:MM)`.
///
/// When the battery has less than 10% charge remaining, the widget's text will
/// change to the specified `warning_color`.
///
/// On Linux, battery charge information is read from [`/sys/class/power_supply/BAT0/`].
///
/// On OpenBSD, battery information is parsed from [`apm`].
///
/// [`/sys/class/power_supply/BAT0/`]: https://www.kernel.org/doc/Documentation/power/power_supply_class.txt
/// [`apm`]: https://man.openbsd.org/apm.8
pub struct Battery {
    update_interval: Duration,
    info: BatteryInfo,
    attr: Attributes,
    warning_color: Color,
}

impl Battery {
    /// Creates a new Battery widget.
    ///
    /// The `warning_color` attributes are used when there is less than 10%
    /// battery charge remaining.
    pub fn new(attr: Attributes, warning_color: Color) -> Self {
        Self {
            update_interval: Duration::from_secs(60),
            info: BatteryInfo::default(),
            attr,
            warning_color,
        }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let info = self.info.load_info()?;

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
            markup: false,
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
