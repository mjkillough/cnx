mod active_window_title;
mod clock;
mod pager;

pub use self::active_window_title::ActiveWindowTitle;
pub use self::clock::Clock;
pub use self::pager::Pager;

use std::time::Duration;

use futures::{Async, Stream, Poll};
use tokio_timer::Timer;
use text::Text;


pub trait Widget {
    fn stream(self: Box<Self>) -> Box<Stream<Item = Vec<Text>, Error = ()>>;
}


pub trait TimerUpdateWidget {
    fn update_interval(&self) -> Duration;
    fn tick(&self) -> Vec<Text>;
}

impl<T: 'static + TimerUpdateWidget> Widget for T {
    #[allow(boxed_local)]
    fn stream(self: Box<Self>) -> Box<Stream<Item = Vec<Text>, Error = ()>> {
        let timer_stream = Timer::default().interval(self.update_interval());
        let text_stream = timer_stream.map(move |_| self.tick());
        Box::new(text_stream.map_err(|_| ()))
    }
}


pub struct WidgetList {
    vec: Vec<Box<Stream<Item = Vec<Text>, Error = ()>>>,
}

impl WidgetList {
    pub fn new(widgets: Vec<Box<Widget>>) -> WidgetList {
        WidgetList { vec: widgets.into_iter().map(|w| w.stream()).collect() }
    }
}

impl Stream for WidgetList {
    type Item = Vec<Option<Vec<Text>>>;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut all_texts: Vec<Option<Vec<Text>>> = Vec::new();
        for stream in &mut self.vec {
            match stream.poll() {
                Ok(Async::Ready(Some(widget_texts))) => all_texts.push(Some(widget_texts)),
                Ok(_) => all_texts.push(None),
                Err(e) => return Err(e),
            }
        }

        if !all_texts.iter().any(|o| o.is_some()) {
            return Ok(Async::NotReady);
        }

        Ok(Async::Ready(Some(all_texts)))
    }
}
