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
    cached: Vec<Option<Vec<Text>>>,
}

impl WidgetList {
    pub fn new(widgets: Vec<Box<Widget>>) -> WidgetList {
        let len = widgets.len();
        WidgetList {
            vec: widgets.into_iter().map(|w| w.stream()).collect(),
            cached: vec![None; len]
        }
    }

    pub fn texts(&self) -> Vec<Text> {
        self.cached
            .clone()
            .into_iter()
            .filter_map(|o| o)
            .flat_map(|v| v)
            .collect()
    }
}

impl Stream for WidgetList {
    type Item = Vec<Text>;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut none_ready = true;

        for (i, stream) in self.vec.iter_mut().enumerate() {
            match stream.poll() {
                Ok(Async::Ready(texts)) => {
                    none_ready = false;
                    self.cached[i] = texts;
                },
                Ok(Async::NotReady) => {},
                Err(e) => return Err(e),
            }
        }

        if none_ready {
            return Ok(Async::NotReady);
        }

        Ok(Async::Ready(Some(self.texts())))
    }
}
