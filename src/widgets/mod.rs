//! Provided widgets and types for creating new widgets.

mod active_window_title;
mod battery;
mod clock;
mod pager;
mod sensors;
#[cfg(feature = "sioctl-volume")]
mod volume;

use std::pin::Pin;

use anyhow::Result;
use tokio::stream::Stream;

use crate::text::Text;

pub use self::active_window_title::ActiveWindowTitle;
pub use self::battery::Battery;
pub use self::clock::Clock;
pub use self::pager::Pager;
pub use self::sensors::Sensors;
#[cfg(feature = "sioctl-volume")]
pub use self::volume::Volume;

/// The stream of `Vec<Text>` returned by each widget.
///
/// This simple type alias makes referring to this stream a little easier. For
/// more information on the stream (and how widgets are structured), please
/// refer to the documentation on the [`Widget`] trait.
///
/// Any errors on the stream are logged but do not affect the runtime of the
/// main [`Cnx`] instance.
///
/// [`Widget`]: trait.Widget.html
/// [`Cnx`]: ../struct.Cnx.html
pub type WidgetStream = Pin<Box<dyn Stream<Item = Result<Vec<Text>>>>>;

/// The main trait implemented by all widgets.
///
/// This simple trait defines a widget. A widget is essentially just a
/// [`tokio::stream::Stream`] and this trait is the standard way of accessing
/// that stream.
///
/// See the [`WidgetStream`] type alias for the exact type of stream that
/// should be returned.
///
/// [`tokio::stream::Stream`]: https://docs.rs/tokio/0.2.18/tokio/stream/trait.Stream.html
/// [`WidgetStream`]: type.WidgetStream.html
pub trait Widget {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream>;
}
