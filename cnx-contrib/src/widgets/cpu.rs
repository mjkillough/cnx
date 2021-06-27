use anyhow::{anyhow, Result};
use cnx::text::{Attributes, Text};
use cnx::widgets::{Widget, WidgetStream};
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

/// Represents CPU widget used to show current CPU consumptiong
pub struct Cpu {
    attr: Attributes,
    cpu_data: CpuData,
    render: Option<Box<dyn Fn(u64) -> String>>,
}

impl Cpu {
    /// Creates a new [`Cpu`] widget.
    ///
    /// Arguments
    ///
    /// * `attr` - Represents `Attributes` which controls properties like
    /// `Font`, foreground and background color etc.
    ///
    /// * `render` - We use the closure to control the way output is
    /// displayed in the bar. `u64` represents the current CPU usage
    /// in percentage.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate cnx;
    /// #
    /// # use cnx::*;
    /// # use cnx::text::*;
    /// # use cnx_contrib::widgets::cpu::*;
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
    /// cnx.add_widget(Cpu::new(attr, None)?);
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(attr: Attributes, render: Option<Box<dyn Fn(u64) -> String>>) -> Result<Self> {
        let cpu_data = CpuData::get_values()?;
        Ok(Cpu {
            attr,
            cpu_data,
            render,
        })
    }

    fn tick(&mut self) -> Result<Vec<Text>> {
        let cpu_data = CpuData::get_values()?;

        // https://github.com/jaor/xmobar/blob/61d075d3c275366c3344d59c058d7dd0baf21ef2/src/Xmobar/Plugins/Monitors/Cpu.hs#L128
        let previous = &self.cpu_data;
        let current = cpu_data;
        let diff_total = (current.user_time - previous.user_time)
            + (current.nice_time - previous.nice_time)
            + (current.system_time - previous.system_time)
            + (current.idle_time - previous.idle_time)
            + (current.iowait_time - previous.iowait_time)
            + (current.total_time - previous.total_time);
        let percentage = match diff_total {
            0 => 0.0,
            _ => (current.total_time - previous.total_time) as f64 / diff_total as f64,
        };

        let cpu_usage = (percentage * 100.0) as u64;
        let text = self
            .render
            .as_ref()
            .map_or(format!("{} %", cpu_usage), |x| (x)(cpu_usage));
        self.cpu_data = current;
        let texts = vec![Text {
            attr: self.attr.clone(),
            text,
            stretch: false,
            markup: true,
        }];
        Ok(texts)
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
        match val[..] {
            [ref user, ref nice, ref system, ref idle, ref iowait, ..] => {
                let user_time = user.parse()?;
                let nice_time = nice.parse()?;
                let system_time = system.parse()?;
                let idle_time = idle.parse()?;
                let iowait_time = iowait.parse()?;
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
        let ten_seconds = Duration::from_secs(10);
        let interval = time::interval(ten_seconds);
        let stream = IntervalStream::new(interval).map(move |_| self.tick());
        Ok(Box::pin(stream))
    }
}
