use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};
use anyhow::{anyhow, Result};
use async_stream::try_stream;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::time::Duration;

pub struct Cpu {
    attr: Attributes,
    cpu_data: CpuData,
}

impl Cpu {
    pub fn new(attr: Attributes) -> Result<Cpu> {
        let cpu_data = CpuData::get_values()?;
        Ok(Cpu { attr, cpu_data })
    }
}

struct CpuData {
    user_time: i64,
    nice_time: i64,
    system_time: i64,
    idle_time: i64,
    total_time: i64,
    iowait_time: i64,
}

impl CpuData {
    fn get_values() -> Result<CpuData> {
        // https://www.kernel.org/doc/Documentation/filesystems/proc.txt
        let file = File::open("/proc/stat")?;
        let mut cpu_line = String::new();
        let mut reader = BufReader::new(file);
        reader.read_line(&mut cpu_line)?;
        let val: Vec<&str> = cpu_line
            .split(' ')
            .filter(|item| item != &"cpu" && !item.is_empty())
            .collect();
        let mut cpu_data = CpuData {
            user_time: 0,
            nice_time: 0,
            system_time: 0,
            idle_time: 0,
            total_time: 0,
            iowait_time: 0,
        };
        println!("{}", val.len());
        match val[..] {
            [ref user, ref nice, ref system, ref idle, ref iowait, ..] => {
                let user_time = user.parse().unwrap();
                let nice_time = nice.parse().unwrap();
                let system_time = system.parse().unwrap();
                let idle_time = idle.parse().unwrap();
                let iowait_time = iowait.parse().unwrap();
                cpu_data.user_time = user_time;
                cpu_data.nice_time = nice_time;
                cpu_data.system_time = system_time;
                cpu_data.idle_time = idle_time;
                cpu_data.iowait_time = iowait_time;
                cpu_data.total_time = user_time + nice_time + system_time;
            }
            _ => return Err(anyhow!("Missing data in /proc/stat")),
        }
        Ok(cpu_data)
    }
}

impl Widget for Cpu {
    fn into_stream(mut self: Box<Self>) -> Result<WidgetStream> {
        let stream = try_stream! {
            loop {
                let cpu_data = CpuData::get_values()?;

                // Based on htop https://stackoverflow.com/a/23376195/1651941
                let prev_idle = self.cpu_data.idle_time;
                let prev_non_idle = self.cpu_data.total_time;

                let idle = cpu_data.idle_time;
                let non_idle = cpu_data.total_time;

                let prev_total = prev_idle + prev_non_idle;
                let total = idle + non_idle;

                // let total_diff = total - prev_total;
                // let idle_diff = idle - prev_idle;
                let total_diff = total;
                let idle_diff = idle;

                // https://github.com/jaor/xmobar/blob/61d075d3c275366c3344d59c058d7dd0baf21ef2/src/Xmobar/Plugins/Monitors/Cpu.hs#L128
                let previous = self.cpu_data;
                let current = cpu_data;
                let diff_total = (current.user_time - previous.user_time) +
                    (current.nice_time - previous.nice_time) +
                    (current.system_time - previous.system_time) +
                    (current.idle_time - previous.idle_time) +
                    (current.iowait_time - previous.iowait_time) +
                    (current.total_time - previous.total_time);
                let percentage = match diff_total {
                    0 => 0.0,
                    _ => (current.total_time - previous.total_time) as f64 / diff_total as f64
                };
                let text = format!("<span foreground=\"#808080\">[</span>Cpu: {}%<span foreground=\"#808080\">]</span>", (percentage * 100.0) as u64);
                let texts = vec![Text {
                    attr: self.attr.clone(),
                    text,
                    stretch: false,
                    markup: true
                }];

                self.cpu_data = current;

                yield texts;

                let sleep_for = Duration::from_secs(10);
                tokio::time::sleep(sleep_for).await;
            }
        };

        Ok(Box::pin(stream))
    }
}
