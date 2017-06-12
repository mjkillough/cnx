extern crate cairo_sys;
extern crate cairo;
extern crate chrono;
extern crate futures;
extern crate mio;
extern crate pango;
extern crate pangocairo;
extern crate tokio_core;
extern crate tokio_timer;
extern crate xcb_util;
extern crate xcb;

use std::rc::Rc;

use futures::{Async, Future, Poll, Stream};
use mio::unix::EventedFd;

mod text;
use text::*;
mod widgets;
use widgets::*;
mod window;
use window::*;


fn main() {
    let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
    let conn = Rc::new(conn);

    let w = Window::new(conn.clone(), screen_idx as usize);

    use tokio_core::reactor::{Core, Handle, PollEvented};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    struct XcbEventStream<'a> {
        conn: Rc<xcb::Connection>,
        poll: PollEvented<EventedFd<'a>>,
        would_block: bool,
    };

    impl<'a> XcbEventStream<'a> {
        fn new(conn: Rc<xcb::Connection>,
               conn_fd: &'a std::os::unix::io::RawFd,
               handle: &Handle)
               -> XcbEventStream<'a> {
            // XXX Lifetime of the connection?
            XcbEventStream {
                conn: conn,
                poll: PollEvented::new(EventedFd(conn_fd), handle).unwrap(),
                would_block: true,
            }
        }
    }

    impl<'a> Stream for XcbEventStream<'a> {
        type Item = xcb::GenericEvent;
        type Error = ();

        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            if self.would_block {
                match self.poll.poll_read() {
                    Async::Ready(()) => self.would_block = false,
                    Async::NotReady => return Ok(Async::NotReady),
                }
            }

            match self.conn.poll_for_event() {
                Some(event) => Ok(Async::Ready(Some(event))),
                None => {
                    self.would_block = true;
                    self.poll.need_read();
                    Ok(Async::NotReady)
                }
            }
        }
    }

    let inactive_attr = Attributes {
        font: pango::FontDescription::from_string("Envy Code R 27"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(10.0, 10.0, 5.0, 5.0),
    };
    let active_attr = inactive_attr.with_bg_color(Some(Color::blue()));


    let conn_fd = unsafe { xcb::ffi::base::xcb_get_file_descriptor(conn.get_raw_conn()) };
    let stream = XcbEventStream::new(conn.clone(), &conn_fd, &handle);

    let widgets: Vec<Box<Widget>> =
        vec![Box::new(Pager::new(active_attr, inactive_attr.clone())) as Box<Widget>,
             Box::new(ActiveWindowTitle::new(inactive_attr.clone())) as Box<Widget>,
             Box::new(Clock::new(inactive_attr.clone())) as Box<Widget>];

    let widget_list = widgets::WidgetList::new(widgets);

    struct MainLoop<'a> {
        xcb_stream: XcbEventStream<'a>,
        widget_list: WidgetList,
        window: Window,
    }

    impl<'a> Future for MainLoop<'a> {
        type Item = ();
        type Error = ();

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            match self.widget_list.poll() {
                Ok(Async::Ready(Some(texts))) => self.window.expose(texts),
                Err(e) => return Err(e),
                _ => {}
            }
            match self.xcb_stream.poll() {
                Ok(Async::Ready(Some(event))) => {
                    let r = event.response_type() & !0x80;
                    match r {
                        xcb::EXPOSE => self.window.expose(self.widget_list.texts()),
                        _ => {}
                    }
                }
                Err(e) => return Err(e),
                _ => {}
            }
            Ok(Async::NotReady)
        }
    }

    let fut = MainLoop {
        xcb_stream: stream,
        widget_list: widget_list,
        window: w,
    };

    core.run(fut).unwrap();
}
