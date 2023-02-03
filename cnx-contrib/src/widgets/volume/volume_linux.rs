use alsa::mixer::{SelemChannelId, SelemId};
use alsa::{self, Mixer, PollDescriptors};
use anyhow::{anyhow, Context, Result};
use cnx::text::{Attributes, Text};
use cnx::widgets::{Widget, WidgetStream};
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::task::Poll;
use tokio::io::unix::AsyncFd;
use tokio_stream::{Stream, StreamExt};

/// Shows the current volume of the default ALSA output.
///
/// This widget shows the current volume of the default ALSA output, or '`M`' if
/// the output is muted.
///
/// The widget uses `alsa-lib` to receive events when the volume changes,
/// avoiding expensive polling. If you do not have `alsa-lib` installed, you
/// can disable the `volume-widget` feature on the `cnx` crate to avoid
/// compiling this widget.
pub struct Volume {
    attr: Attributes,
}

impl Volume {
    /// Creates a new Volume widget.
    ///
    /// Creates a new `Volume` widget, whose text will be displayed
    /// with the given [`Attributes`].
    ///
    /// The [`Cnx`] instance is borrowed during construction in order to get
    /// access to handles of its event loop. However, it is not borrowed for the
    /// lifetime of the widget. See the [`cnx_add_widget!()`] for more discussion
    /// about the lifetime of the borrow.
    ///
    /// [`Attributes`]: ../text/struct.Attributes.html
    /// [`Cnx`]: ../struct.Cnx.html
    /// [`cnx_add_widget!()`]: ../macro.cnx_add_widget.html
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
    /// # use cnx_contrib::widgets::*;
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
    /// cnx.add_widget(volume::Volume::new(attr.clone()));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(attr: Attributes) -> Volume {
        Volume { attr }
    }
}

// https://github.com/mjkillough/cnx/blob/92c24238be541c75d88181208862505739be33fd/src/widgets/volume.rs

impl Widget for Volume {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let mixer_name = "default";
        // We don't attempt to use the same mixer to listen for events and to
        // recompute the mixer state (in the callback below) as the Mixer seems
        // to cache the state from when it was created. It's relatively cheap
        // create a new mixer each time we get an event though.
        let mixer = Mixer::new(mixer_name, true)
            .with_context(|| format!("Failed to open ALSA mixer: {mixer_name}"))?;
        let stream = AlsaEventStream::new(mixer)?.map(move |()| {
            // FrontLeft has special meaning in ALSA and is the channel
            // that's used when the mixer is mono.
            let channel = SelemChannelId::FrontLeft;

            let mixer = Mixer::new(mixer_name, true)?;
            let master = mixer.find_selem(&SelemId::new("Master", 0))
             .ok_or_else(|| anyhow!("Couldn't open Master channel"))?;

            let mute = master.get_playback_switch(channel)? == 0;

            let text = if !mute {
                let volume = master.get_playback_volume(channel)?;
                let (min, max) = master.get_playback_volume_range();
                let percentage = (volume as f64 / (max as f64 - min as f64)) * 100.0;
                format!("<span foreground=\"#808080\">[</span>🔈 {percentage:.0}%<span foreground=\"#808080\">]</span>")
            } else {
                "🔇".to_owned()
            };

            Ok(vec![Text {
                attr: self.attr.clone(),
                text,
                stretch: false,
                markup: true,
            }])
        });

        Ok(Box::pin(stream))
    }
}

struct AlsaEvented(Mixer);

impl AlsaEvented {
    fn mixer(&self) -> &Mixer {
        &self.0
    }

    fn fds(&self) -> Vec<RawFd> {
        self.0.get().map_or(vec![], |vec_poll| {
            vec_poll.iter().map(|pollfd| pollfd.fd).collect()
        })
    }
}

struct AlsaEventStream {
    poll: AsyncFd<AlsaEvented>,
    initial: bool,
}

impl AsRawFd for AlsaEvented {
    fn as_raw_fd(&self) -> RawFd {
        self.fds()
            .into_iter()
            .next()
            .expect("volume: as_raw_fd empty")
    }
}

impl AlsaEventStream {
    fn new(mixer: Mixer) -> Result<AlsaEventStream> {
        Ok(AlsaEventStream {
            poll: AsyncFd::new(AlsaEvented(mixer))?,
            // The first few calls to poll() need to process any existing events.
            // We don't know what state the fds are in when we give them to tokio
            // and it's edge-triggered.
            initial: true,
        })
    }
}

impl Stream for AlsaEventStream {
    // We don't bother yielding the events and just yield unit each time we get
    // an event. This stream is used only to get woken up when the ALSA state
    // changes - the caller is expected to requery all necessary state when
    // it receives a new item from the stream.
    type Item = ();

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> Poll<Option<Self::Item>> {
        // Always assume we're ready initially, so that we can clear the
        // state of the fds.

        // Do a poll with a timeout of 0 to figure out exactly which fds were
        // woken up, followed by a call to revents() which clears the pending
        // events. We don't actually care what the events are - we're just
        // using it as a wake-up so we can check the volume again.
        if self.initial {
            let mixer = self.poll.get_ref().mixer();
            let _poll_result = alsa::poll::poll_all(&[mixer], 0);
            self.initial = false;
            return Poll::Ready(Some(()));
        }
        // All events have been consumed - tell Tokio we're interested in waiting
        // for more again.
        match self.poll.poll_read_ready(cx) {
            Poll::Ready(Ok(mut r)) => {
                let mixer = self.poll.get_ref().mixer();
                let _poll_result = alsa::poll::poll_all(&[mixer], 0);
                let _result = mixer.handle_events();
                r.clear_ready();
                Poll::Ready(Some(()))
            }
            Poll::Ready(Err(_)) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
