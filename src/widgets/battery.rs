use std::f64;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::time::Duration;

use text::{Attributes, Text};


enum Status {
    Full,
    Charging,
    Discharging,
    Unknown,
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Full" => Ok(Status::Full),
            "Charging" => Ok(Status::Charging),
            "Discharging" => Ok(Status::Discharging),
            "Unknown" => Ok(Status::Unknown),
            _ => Err(format!("Unknown Status: {}", s)),
        }
    }
}


pub struct Battery {
    update_interval: Duration,
    battery: String,
    attr: Attributes,
}

impl Battery {
    pub fn new(attr: Attributes) -> Battery {
        Battery {
            update_interval: Duration::from_secs(60),
            battery: "BAT0".to_owned(),
            attr: attr,
        }
    }

    fn load_value<T>(&self, file: &str) -> T
        where T: FromStr,
              // XXX Remove once we get rid of unwrap():
              T::Err: Debug
    {
        let path = format!("/sys/class/power_supply/{}/{}", self.battery, file);
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        FromStr::from_str(contents.trim()).unwrap()
    }

    fn tick(&self) -> Vec<Text> {
        let full: f64 = self.load_value("energy_full");
        let now: f64 = self.load_value("energy_now");
        let percentage = (now / full) * 100.0;

        // If we're discharging, show time to empty.
        // If we're charging, show time to full.
        let power: f64 = self.load_value("power_now");
        let status: Status = self.load_value("status");
        let time = match status {
            Status::Discharging => now / power,
            Status::Charging => (full - now) / power,
            _ => 0.0,
        };
        let hours = time as u64;
        let minutes = (time * 60.0) as u64 % 60;


        let text = format!("({percentage:.0}% - {hours}:{minutes:02})",
                           percentage = percentage,
                           hours = hours,
                           minutes = minutes);

        vec![Text {
                 attr: self.attr.clone(),
                 text: text,
                 stretch: false,
             }]
    }
}

timer_widget!(Battery, update_interval, tick);
