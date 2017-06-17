use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use regex::Regex;

use errors::*;
use text::{Attributes, Text};


#[derive(Debug, PartialEq)]
struct Value<'a> {
    temp: &'a str,
    units: &'a str,
}

fn parse_sensors_output<'a>(output: &'a str) -> Result<HashMap<&'a str, Value<'a>>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            // Note: we ignore + but capture -
            r"\n(?P<name>[\w ]+):\s+\+?(?P<temp>-?\d+\.\d+).(?P<units>[C|F])"
        ).expect("Failed to compile regex for parsing sensors output");
    }

    let mut map = HashMap::new();
    for mat in RE.captures_iter(output) {
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


pub struct Sensors {
    update_interval: Duration,
    attr: Attributes,
    sensors: Vec<String>,
}

impl Sensors {
    pub fn new(attr: Attributes, sensors: Vec<String>) -> Sensors {
        Sensors {
            update_interval: Duration::from_secs(60),
            attr,
            sensors,
        }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let output = Command::new("sensors")
            .output()
            .chain_err(|| "Failed to run `sensors`")?;
        let string = String::from_utf8(output.stdout)
            .chain_err(|| "Invalid UTF-8 in sensors output")?;
        let parsed = parse_sensors_output(&string)
            .chain_err(|| "Failed to parse `sensors` output")?;
        self.sensors
            .iter()
            .map(|sensor_name| {
                let text = parsed.get::<str>(&sensor_name).map_or(
                    "?".to_owned(),
                    |&Value { temp, units }| {
                        format!("{}°{}", temp, units)
                    },
                );
                Ok(Text {
                    attr: self.attr.clone(),
                    text: text,
                    stretch: false,
                })
            })
            .collect()
    }
}

timer_widget!(Sensors, update_interval, tick);


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
