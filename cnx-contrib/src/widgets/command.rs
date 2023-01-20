use anyhow::Result;
use cnx::text::{Attributes, Text};
use cnx::widgets::{Widget, WidgetStream};
use std::process::Command as Process;
use std::time::Duration;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

pub struct Command {
    attr: Attributes,
    command: String,
}

impl Command {
    /// Creates a new [`Command`] widget.
    ///
    /// Arguments
    ///
    /// * `attr` - Represents `Attributes` which controls properties like
    /// `Font`, foreground and background color etc.
    ///
    /// * `command` - Command to be executed.
    ///
    /// # Examples
    ///
    /// ```
    /// #[macro_use]
    /// extern crate cnx;
    ///
    /// use cnx::*;
    /// use cnx::text::*;
    /// use cnx_contrib::widgets::command::*;
    /// use anyhow::Result;
    ///
    /// fn run() -> Result<()> {
    /// let attr = Attributes {
    ///     font: Font::new("SourceCodePro 16"),
    ///     fg_color: Color::white(),
    ///     bg_color: None,
    ///     padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    /// };
    ///
    /// let mut cnx = Cnx::new(Position::Top);
    /// cnx.add_widget(Command::new(attr, "echo foo".into()));
    /// Ok(())
    /// }
    /// fn main() { run().unwrap(); }
    /// ```
    pub fn new(attr: Attributes, command: String) -> Self {
        Self { attr, command }
    }

    fn tick(&self) -> Vec<Text> {
        let output = Process::new("sh")
            .arg("-c")
            .arg(self.command.clone())
            .output()
            .expect("failed to execute process");

        let texts = vec![Text {
            attr: self.attr.clone(),
            text: String::from_utf8(output.stdout).unwrap_or_else(|_| "error".into()),
            stretch: false,
            markup: true,
        }];

        texts
    }
}

impl Widget for Command {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let ten_seconds = Duration::from_secs(10);
        let interval = time::interval(ten_seconds);
        let stream = IntervalStream::new(interval).map(move |_| Ok(self.tick()));

        Ok(Box::pin(stream))
    }
}
