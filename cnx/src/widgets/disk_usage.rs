use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};
use anyhow::Result;
use byte_unit::{Byte, ByteUnit};
use nix::sys::statvfs::statvfs;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

#[derive(Debug)]
pub struct DiskInfo {
    pub total: Byte,
    pub used: Byte,
    pub free: Byte,
}

impl DiskInfo {
    pub fn new(path: &str) -> Result<Self> {
        let stat = statvfs(path)?;
        let total_size = stat.blocks() * stat.fragment_size();
        let used = (stat.blocks() - stat.blocks_free()) * stat.fragment_size();
        let available = stat.blocks_available() * stat.fragment_size();
        let total = byte_unit::Byte::from_bytes(total_size as u128);
        let used = byte_unit::Byte::from_bytes(used as u128);
        let free: Byte = byte_unit::Byte::from_bytes(available as u128);

        let disk_info = DiskInfo { total, used, free };
        Ok(disk_info)
    }
}

pub struct DiskUsage {
    attr: Attributes,
    path: String,
    render: Option<Box<dyn Fn(DiskInfo) -> String>>,
}

impl DiskUsage {
    pub fn new(
        attr: Attributes,
        path: String,
        render: Option<Box<dyn Fn(DiskInfo) -> String>>,
    ) -> Self {
        Self { attr, render, path }
    }

    fn tick(&self) -> Result<Vec<Text>> {
        let disk_info = DiskInfo::new(self.path.as_ref())?;
        let disk_default_str = format!(
            "Disk: {}/{}",
            disk_info.used.get_adjusted_unit(ByteUnit::GiB).format(0),
            disk_info.total.get_adjusted_unit(ByteUnit::GiB).format(0)
        );

        let text: String = self
            .render
            .as_ref()
            .map_or(disk_default_str, |disk| (disk)(disk_info));
        let texts = vec![Text {
            attr: self.attr.clone(),
            text,
            stretch: false,
            markup: true,
        }];
        Ok(texts)
    }
}

impl Widget for DiskUsage {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let one_hour = Duration::from_secs(3600);
        let interval = time::interval(one_hour);
        let stream = IntervalStream::new(interval).map(move |_| self.tick());

        Ok(Box::pin(stream))
    }
}
