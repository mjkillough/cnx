use futures::{Async, Stream, Poll};

use text::Text;


pub type WidgetStream = Box<Stream<Item = Vec<Text>, Error = ()>>;

pub trait Widget {
    fn stream(self: Box<Self>) -> WidgetStream;
}


macro_rules! timer_widget {
    ($widget:ty, $interval:ident, $tick:ident) => {
        impl ::widgets::Widget for $widget {
            #[allow(boxed_local)]
            fn stream(self: Box<Self>) -> ::widgets::WidgetStream {
                use futures::{stream, Stream};
                use tokio_timer::Timer;

                // The Timer will only fire after the first interval. To avoid
                // waiting for the initial state, call the tick ourselves.
                let initial = stream::once::<_, ()>(Ok(self.$tick()));

                let timer_stream = Timer::default().interval(self.$interval);
                let text_stream = timer_stream.map(move |_| self.$tick()).map_err(|_| ());

                Box::new(initial.chain(text_stream))
            }
        }
    }
}


macro_rules! x_properties_widget {
    ($widget:ty, $handle:ident, $on_change:ident; [ $( $property:ident ),+ ])  => {
        impl ::widgets::Widget for $widget {
            #[allow(boxed_local)]
            fn stream(self: Box<Self>) -> ::widgets::WidgetStream {
                use std::rc::Rc;

                use futures::{stream, Stream};
                use xcb;
                use xcb::xproto::{PropertyNotifyEvent, PROPERTY_NOTIFY};

                use bar::XcbEventStream;

                let (xcb_conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
                let root_window = xcb_conn
                    .get_setup()
                    .roots()
                    .nth(screen_idx as usize)
                    .expect("Invalid screen")
                    .root();
                let ewmh_conn = ewmh::Connection::connect(xcb_conn)
                    .map_err(|_| ())
                    .unwrap();
                let conn = Rc::new(ewmh_conn);

                let properties = [ $( conn.$property() ),+ ];

                // Register for all PROPERTY_CHANGE events. We'll filter out the ones
                // that are interesting below.
                let attributes = [(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_PROPERTY_CHANGE)];
                xcb::change_window_attributes(&conn, root_window, &attributes);
                conn.flush();

                // Pretend there was an initial property change to get the initial
                // contents of the widget, then allow our stream of XCB events to
                // call the callback for actual changes.
                let initial = stream::once::<_, ()>(Ok(self.$on_change(&conn, screen_idx)));

                let xcb_stream = XcbEventStream::new(conn.clone(), &self.$handle);
                let text_stream = xcb_stream.filter_map(move |event| {
                    if event.response_type() == PROPERTY_NOTIFY {
                        let event: &PropertyNotifyEvent = xcb::cast_event(&event);
                        if properties.iter().any(|p| *p == event.atom()) {
                            return Some(self.$on_change(&conn, screen_idx));
                        }
                    }
                    None
                });

                Box::new(initial.chain(text_stream))
            }
        }
    }
}


// Defined after macros because of macro scoping rules:
mod active_window_title;
mod battery;
mod clock;
mod pager;
mod volume;

pub use self::active_window_title::ActiveWindowTitle;
pub use self::battery::Battery;
pub use self::clock::Clock;
pub use self::pager::Pager;
pub use self::volume::Volume;


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
