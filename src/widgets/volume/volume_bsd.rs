use std::u8;

use anyhow::Result;
use async_stream::stream;
use sioctl::Sioctl;
use tokio::stream::{self, Stream, StreamExt};
use tokio::sync::mpsc;

use crate::text::{Attributes, Text};
use crate::widgets::{Widget, WidgetStream};

#[derive(Copy, Clone, Debug, PartialEq)]
enum State {
    Unknown,
    Muted,
    Unmuted { percentage: f32 },
}

struct OpenBsd;

impl OpenBsd {
    fn new() -> Self {
        Self
    }

    fn stream(self) -> impl Stream<Item = State> {
        // Grab initial state before starting to watch for changes.
        let sioctl = Sioctl::new();
        let controls = sioctl.controls();

        let (sender, receiver) = mpsc::unbounded_channel();
        let watcher = sioctl.watch(move |control| {
            if let Err(error) = sender.send(control.clone()) {
                println!("Error sending sioctl message: {}", error);
            }
        });

        let mut stream = stream::iter(controls).chain(receiver);
        stream! {
            // Move watcher into stream! {} to keep it alive.
            let watcher = watcher;
            let mut state = State::Unknown;
            let mut muted = false;
            let mut percentage = 1.0;

            loop {
                if let Some(control) = stream.next().await {
                    let name = control.name.as_ref();
                    let func = control.func.as_ref();
                    let value = control.value;

                    match (name, func, value) {
                        ("output", "mute", 1) => muted = true,
                        ("output", "mute", 0) => muted = false,
                        ("output", "level", _) => percentage = self.percentage(value),
                        _ => (),
                    }

                    let new = match (muted, percentage) {
                        (true, _) => State::Muted,
                        (_, percentage) => State::Unmuted { percentage },
                    };

                    if state != new {
                        state = new;
                        yield state;
                    }
                }
            }
        }
    }

    fn percentage(&self, value: u8) -> f32 {
        (f32::from(value) / f32::from(u8::MAX)) * 100.0
    }
}

type VolumeInfo = OpenBsd;

pub struct Volume {
    attr: Attributes,
}

impl Volume {
    /// Creates a new Volume widget.
    pub fn new(attr: Attributes) -> Self {
        Self { attr }
    }

    fn on_change(&self, state: State) -> Result<Vec<Text>> {
        let text = match state {
            State::Unknown => "?".to_owned(),
            State::Muted => "M".to_owned(),
            State::Unmuted { percentage } => format!("{:.0}%", percentage),
        };

        Ok(vec![Text {
            attr: self.attr.clone(),
            text,
            stretch: false,
        }])
    }
}

impl Widget for Volume {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let info = VolumeInfo::new();
        let stream = info.stream().map(move |state| self.on_change(state));

        Ok(Box::pin(stream))
    }
}
