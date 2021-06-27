//! Provided widgets and types for creating new widgets.

mod active_window_title;

mod clock;
mod pager;
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
/// main [`crate::Cnx`] instance.
///
pub type WidgetStream = Pin<Box<dyn Stream<Item = Result<Vec<Text>>>>>;

/// The main trait implemented by all widgets.
///
/// This simple trait defines a widget. A widget is essentially just a
/// [`futures::stream::Stream`] and this trait is the standard way of accessing
/// that stream.
///
/// See the [`WidgetStream`] type alias for the exact type of stream that
/// should be returned.
///
pub trait Widget {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream>;
}
