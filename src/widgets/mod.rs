//! Provided widgets and types for creating new widgets.

mod active_window_title;

#[cfg(target_os = "openbsd")]
mod battery_bsd;
#[cfg(target_os = "openbsd")]
mod sensors_bsd;
#[cfg(target_os = "openbsd")]
pub use self::sensors_battery::Battery;

#[cfg(target_os = "linux")]
mod battery_linux;
#[cfg(target_os = "linux")]
mod sensors_linux;
#[cfg(target_os = "linux")]
pub use self::battery_linux::Battery;
#[cfg(target_os = "linux")]
pub use self::sensors_linux::Sensors;
mod clock;
pub mod cpu;
pub mod disk_usage;
mod pager;
pub mod volume;
pub mod weather;
pub mod wireless;
pub use self::active_window_title::ActiveWindowTitle;
pub use self::clock::Clock;
pub use self::pager::Pager;
use crate::text::Text;
use anyhow::Result;
use futures::stream::Stream;
use std::pin::Pin;

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
