use std::io;
use std::os::unix::io::RawFd;
use std::rc::Rc;

use cairo_sys;
use cairo::{self, Context, XCBSurface};
use futures::{future, Async, Future, Poll, Stream};
use futures::stream::MergedItem;
use mio::{self, PollOpt, Ready, Token};
use mio::event::Evented;
use mio::unix::EventedFd;
use tokio_core::reactor::{Handle, PollEvented};
use xcb;

use text::{Color, Text};
use widgets::{WidgetList, Widget};


fn get_root_visual_type(conn: &xcb::Connection, screen: &xcb::Screen) -> xcb::Visualtype {
    for root in conn.get_setup().roots() {
        for allowed_depth in root.allowed_depths() {
            for visual in allowed_depth.visuals() {
                if visual.visual_id() == screen.root_visual() {
                    return visual;
                }
            }
        }
    }
    panic!("No visual type found");
}


/// Creates a `cairo::Surface` for the XCB window with the given `id`.
fn cairo_surface_for_xcb_window(conn: &xcb::Connection,
                                screen: &xcb::Screen,
                                id: u32,
                                width: i32,
                                height: i32)
                                -> cairo::Surface {
    let cairo_conn = unsafe {
        cairo::XCBConnection::from_raw_none(conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t)
    };
    let visual = unsafe {
        cairo::XCBVisualType::from_raw_none(&mut get_root_visual_type(&conn, &screen).base as
                                            *mut xcb::ffi::xcb_visualtype_t as
                                            *mut cairo_sys::xcb_visualtype_t)
    };
    let drawable = cairo::XCBDrawable(id);
    cairo::Surface::create(&cairo_conn, &drawable, &visual, width, height)
}


pub struct Bar {
    conn: Rc<xcb::Connection>,
    window_id: u32,
    screen_idx: usize,
    surface: cairo::Surface,
    contents: Vec<Vec<Text>>,
}

impl Bar {
    pub fn new() -> Bar {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
        let screen_idx = screen_idx as usize;
        let id = conn.generate_id();

        let surface = {
            let screen = conn.get_setup()
                .roots()
                .nth(screen_idx)
                .expect("invalid screen");
            let values = [(xcb::CW_BACK_PIXEL, screen.black_pixel()),
                          (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE)];

            let (width, height) = (screen.width_in_pixels(), 100);

            xcb::create_window(&conn,
                               xcb::COPY_FROM_PARENT as u8,
                               id,
                               screen.root(),
                               0,
                               0,
                               width,
                               height,
                               10,
                               xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
                               screen.root_visual(),
                               &values);

            cairo_surface_for_xcb_window(&conn, &screen, id, width as i32, height as i32)
        };

        let bar = Bar {
            conn: Rc::new(conn),
            window_id: id,
            screen_idx,
            surface,
            contents: Vec::new(),
        };
        bar.map_window();
        bar.flush();
        bar
    }

    fn flush(&self) {
        self.conn.flush();
    }

    fn map_window(&self) {
        xcb::map_window(&self.conn, self.window_id);
    }

    fn screen(&self) -> xcb::Screen {
        self.conn
            .get_setup()
            .roots()
            .nth(self.screen_idx)
            .expect("Invalid screen")
    }

    fn update_contents(&mut self, new_contents: Vec<Option<Vec<Text>>>) {
        for (i, opt) in new_contents.into_iter().enumerate() {
            if let Some(new) = opt {
                self.contents[i] = new;
            }
        }
    }

    fn render_text_blocks(&self, texts: Vec<Text>) {
        // Layout each block of text. After this, we can query the width of each
        // block, which will allow us to do more complex layout below.
        let mut layouts: Vec<_> = texts.into_iter().map(|t| t.layout(&self.surface)).collect();

        // Calculate how much free space we have after laying out all the
        // non-stretch blocks. Split the remaining space (if any) between the
        // stretch blocks. If there isn't enough space for the non-stretch blocks
        // do nothing and allow it to overflow.
        {
            let mut width = 0.0;
            let mut stretched = Vec::new();
            for layout in &mut layouts {
                if !layout.stretch() {
                    width += layout.width();
                } else {
                    stretched.push(layout);
                }
            }

            let remaining_width = self.screen().width_in_pixels() as f64 - width;
            let remaining_width = if remaining_width < 0.0 {
                0.0
            } else {
                remaining_width
            };
            let width_per_stretched = remaining_width / (stretched.len() as f64);
            for layout in &mut stretched {
                layout.set_width(width_per_stretched);
            }
        }

        // TODO: Set the height of the window and the height of each text block(?)
        // to the height of the largest bit of text.

        // Finally, just render each block of text in turn.
        let mut x = 0.0;
        for layout in &layouts {
            layout.render(x, 0.0);
            x += layout.width();
        }
    }

    fn expose(&self) {
        // Clear to black before re-painting.
        let context = Context::new(&self.surface);
        Color::black().apply_to_context(&context);
        context.paint();

        let flattened_contents = self.contents.clone().into_iter().flat_map(|v| v).collect();

        self.render_text_blocks(flattened_contents);
        self.conn.flush();
    }

    pub fn run_event_loop(mut self,
                          handle: &Handle,
                          widgets: Vec<Box<Widget>>)
                          -> Box<Future<Item = (), Error = ()>> {
        self.contents = vec![Vec::new(); widgets.len()];

        let events_stream = XcbEventStream::new(self.conn.clone(), handle);
        let widget_updates_stream = WidgetList::new(widgets);

        let event_loop = events_stream.merge(widget_updates_stream);
        let fut = event_loop.for_each(move |item| {
            let (xcb_event, widget_update) = match item {
                MergedItem::First(e) => (Some(e), None),
                MergedItem::Second(u) => (None, Some(u)),
                MergedItem::Both(e, u) => (Some(e), Some(u)),
            };

            if let Some(update) = widget_update {
                self.update_contents(update);
                self.expose();
            }
            if let Some(event) = xcb_event {
                match event.response_type() & !0x80 {
                    xcb::EXPOSE => self.expose(),
                    _ => {}
                }
            }

            future::ok(())
        });

        Box::new(fut)
    }
}


struct XcbEvented(Rc<xcb::Connection>);

impl XcbEvented {
    fn fd(&self) -> RawFd {
        unsafe { xcb::ffi::base::xcb_get_file_descriptor(self.0.get_raw_conn()) }
    }
}

impl Evented for XcbEvented {
    fn register(&self,
                poll: &mio::Poll,
                token: Token,
                interest: Ready,
                opts: PollOpt)
                -> io::Result<()> {
        EventedFd(&self.fd()).register(poll, token, interest, opts)
    }

    fn reregister(&self,
                  poll: &mio::Poll,
                  token: Token,
                  interest: Ready,
                  opts: PollOpt)
                  -> io::Result<()> {
        EventedFd(&self.fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.fd()).deregister(poll)
    }
}


struct XcbEventStream {
    conn: Rc<xcb::Connection>,
    poll: PollEvented<XcbEvented>,
    would_block: bool,
}

impl XcbEventStream {
    fn new(conn: Rc<xcb::Connection>, handle: &Handle) -> XcbEventStream {
        let evented = XcbEvented(conn.clone());
        XcbEventStream {
            conn,
            poll: PollEvented::new(evented, handle).unwrap(),
            would_block: true,
        }
    }
}

impl Stream for XcbEventStream {
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
