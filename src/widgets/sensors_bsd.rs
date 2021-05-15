#[cfg(target_os = "openbsd")]
use anyhow::{Context, Error, Result};
use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use regex::Regex;

use crate::cmd::command_output;
use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};
use lazy_static::lazy_static;
// use regex::Regex;
use std::str::FromStr;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

#[derive(Debug, PartialEq)]
struct Value {
    value: u64,
    units: String,
}

#[derive(Default)]
struct OpenBsd;

impl OpenBsd {
    fn parse_value(&self, value: &str) -> Result<u64> {
        let value = f64::from_str(value)?;
        Ok(value as u64)
    }

    fn parse_units(&self, units: &str) -> String {
        match units {
            "degC" => "°C".to_owned(),
            "RPM" => " RPM".to_owned(),
            _ => units.to_owned(),
        }
    }

    fn load_values(&self, sensors: &[String]) -> Result<Vec<Value>> {
        // TODO: Use sysctl C API rather than shelling out.

        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"(?P<name>[^=]+)=(?P<value>[^ ]+) (?P<units>[^ \n]+).*\n")
                    .expect("Failed to compile Sensors regex");
        }

        let output = command_output("sysctl", sensors)?;
        let values = RE
            .captures_iter(&output)
            .map(|mat| {
                let value = mat
                    .name("value")
                    .ok_or_else(|| anyhow!("Missing value in Sensors output"))?;
                let units = mat
                    .name("units")
                    .ok_or_else(|| anyhow!("Missing units in Sensors output"))?;

                let value = self.parse_value(value.as_str())?;
                let units = self.parse_units(units.as_str());

                Ok(Value { value, units })
            })
            .collect::<Result<_>>()?;

        Ok(values)
    }
}

// TODO: Use config flag when we re-implement Linux version.
type SensorsInfo = OpenBsd;

/// Shows the value from one or more hardware sensors.
///
/// On Linux, this shows the temperature reported by one or more sensors from the
/// output of the `sensors` command, which is part of the [`lm_sensors`]
/// package. It expects the `sensors` executable to be available in the `PATH`.
///
/// On OpenBSD, this shows the values reported by one or more sensors available
/// through [`sysctl`].
///
/// [`lm_sensors`]: https://wiki.archlinux.org/index.php/lm_sensors
/// [`sysctl`]: https://man.openbsd.org/sysctl.8
pub struct Sensors {
    update_interval: Duration,
    attr: Attributes,
    sensors: Vec<String>,
    info: SensorsInfo,
}

impl Sensors {
    /// Creates a new Sensors widget.
    ///
    /// A list of sensor names should be passed as the `sensors` argument.
    pub fn new<S: Into<String>>(attr: Attributes, sensors: Vec<S>) -> Sensors {
        let sensors = sensors.into_iter().map(Into::into).collect();
        Sensors {
            update_interval: Duration::from_secs(60),
            attr,
            sensors,
            info: SensorsInfo::default(),
        }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let values = self
            .info
            .load_values(&self.sensors)
            .context("Failed to get sensor information")?;

        let texts = values
            .into_iter()
            .map(|Value { value, units }| {
                let text = format!("{}{}", value, units);
                Text {
                    attr: self.attr.clone(),
                    text,
                    stretch: false,
                    markup: false,
                }
            })
            .collect();

        Ok(texts)
    }
}

impl Widget for Sensors {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let interval = time::interval(self.update_interval);
        let stream = IntervalStream::new(interval).map(move |_| self.tick());

        Ok(Box::pin(stream))
    }
}
