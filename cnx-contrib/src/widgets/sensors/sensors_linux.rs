use anyhow::{anyhow, Context, Result};
#[cfg(target_os = "linux")]
use cnx::text::{Attributes, Text};
use cnx::widgets::{Widget, WidgetStream};
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

#[derive(Debug, PartialEq)]
struct Value<'a> {
    temp: &'a str,
    units: &'a str,
}

/// Parses the output of the `sensors` executable from `lm_sensors`.
fn parse_sensors_output(output: &str) -> Result<HashMap<&str, Value<'_>>> {
    let re: Regex = Regex::new(
        // Note: we ignore + but capture -
        r"\n(?P<name>[\w ]+):\s+\+?(?P<temp>-?\d+\.\d+).(?P<units>[C|F])",
    )
    .map_err(|_| anyhow!("Failed to compile regex for parsing sensors output"))?;

    let mut map = HashMap::new();
    for mat in re.captures_iter(output) {
        // These .unwraps() are harmless. If we have a match, we have these groups.
        map.insert(
            mat.name("name").unwrap().as_str(),
            Value {
                temp: mat.name("temp").unwrap().as_str(),
                units: mat.name("units").unwrap().as_str(),
            },
        );
    }

    Ok(map)
}

/// Shows the temperature from one or more sensors.
///
/// This widget shows the temperature reported by one or more sensors from the
/// output of the `sensors` command, which is part of the [`lm_sensors`]
/// package.
///
/// It expects the `sensors` executable to be available in the `PATH`.
///
/// [`lm_sensors`]: https://wiki.archlinux.org/index.php/lm_sensors
pub struct Sensors {
    update_interval: Duration,
    attr: Attributes,
    sensors: Vec<String>,
}

impl Sensors {
    /// Creates a new Sensors widget.
    ///
    /// Creates a new `Sensors` widget, whose text will be displayed with the
    /// given [`Attributes`].
    ///
    /// A list of sensor names should be passed as the `sensors` argument. (You
    /// can discover the names by running the `sensors` utility in a terminal).
    ///
    /// The [`cnx::Cnx`] instance is borrowed during construction in order to get
    /// access to handles of its event loop. However, it is not borrowed for the
    /// lifetime of the widget. See the [`cnx::Cnx::add_widget`] for more discussion
    /// about the lifetime of the borrow.
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
    /// # use cnx_contrib::widgets::sensors::*;
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
    /// cnx.add_widget(
    ///     Sensors::new(attr.clone(), vec!["Core 0", "Core 1"])
    /// );
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new<S: Into<String>>(attr: Attributes, sensors: Vec<S>) -> Sensors {
        Sensors {
            update_interval: Duration::from_secs(60),
            attr,
            sensors: sensors.into_iter().map(Into::into).collect(),
        }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let output = Command::new("sensors")
            .output()
            .context("Failed to run `sensors`")?;
        let string = String::from_utf8(output.stdout).context("Invalid UTF-8 in sensors output")?;
        let parsed = parse_sensors_output(&string).context("Failed to parse `sensors` output")?;
        self.sensors
            .iter()
            .map(|sensor_name| {
                let text = parsed
                    .get::<str>(sensor_name)
                    .map_or("Invalid".to_owned(), |&Value { temp, units }| {
                        format!("{temp}°{units}")
                    });
                Ok(Text {
                    attr: self.attr.clone(),
                    text,
                    stretch: false,
                    markup: false,
                })
            })
            .collect()
    }
}

impl Widget for Sensors {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let interval = time::interval(self.update_interval);
        let stream = IntervalStream::new(interval).map(move |_| self.tick());

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod test {
    use super::{parse_sensors_output, Value};

    #[test]
    fn works() {
        let output = r#"applesmc-isa-0300
Adapter: ISA adapter
Right Side  :    0 RPM  (min = 2000 RPM, max = 6199 RPM)
Ts1S:         -127.0 C
Ts2S:          +34.0 F

coretemp-isa-0000
Adapter: ISA adapter
Package id 0:  +58.0 C  (high = +105.0 C, crit = +105.0 C)
Core 0:        +53.0 C  (high = +105.0 C, crit = +105.0 C)
Core 1:        +58.0 C  (high = +105.0 C, crit = +105.0 C)
"#;

        let parsed = parse_sensors_output(output).unwrap();
        assert_eq!(
            parsed.get("Core 0"),
            Some(&Value {
                temp: "53.0",
                units: "C",
            })
        );
        assert_eq!(
            parsed.get("Core 1"),
            Some(&Value {
                temp: "58.0",
                units: "C",
            })
        );
        assert_eq!(
            parsed.get("Ts1S"),
            Some(&Value {
                temp: "-127.0",
                units: "C",
            })
        );
        assert_eq!(
            parsed.get("Ts2S"),
            Some(&Value {
                temp: "34.0",
                units: "F",
            })
        );

        assert_eq!(parsed.len(), 5);
    }
}
