extern crate cairo_sys;
extern crate cairo;
extern crate chrono;
extern crate mio;
extern crate pango;
extern crate pangocairo;
extern crate xcb_util;
extern crate xcb;

use std::rc::Rc;

use cairo::{Context, Surface, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use mio::{Events, Ready, Poll, PollOpt, Token};
use mio::unix::EventedFd;
use pango::LayoutExt;
use pangocairo::CairoContextExt;
use xcb::ffi::*;

mod text;
use text::*;
mod widgets;
use widgets::*;


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

// TODO: impl Drop?
struct Window {
    conn: Rc<xcb::Connection>,
    screen_idx: usize,
    id: u32,
    surface: Surface,
}

impl Window {
    fn new(conn: Rc<xcb::Connection>, screen_idx: usize) -> Window {
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

            let surface = unsafe {
                let cairo_conn = XCBConnection::from_raw_none(conn.get_raw_conn() as
                                                              *mut cairo_sys::xcb_connection_t);
                let visual =
                    XCBVisualType::from_raw_none(&mut get_root_visual_type(&conn, &screen).base as
                                                 *mut xcb::ffi::xcb_visualtype_t as
                                                 *mut cairo_sys::xcb_visualtype_t);
                let drawable = XCBDrawable(id);
                Surface::create(&cairo_conn, &drawable, &visual, width as i32, height as i32)
                // TODO: Update surface width/height when window size changes.
            };

            surface
        };

        xcb::map_window(&conn, id);
        conn.flush();

        Window {
            conn,
            screen_idx,
            id,
            surface,
        }
    }

    fn screen<'a>(&'a self) -> xcb::Screen<'a> {
        self.conn
            .get_setup()
            .roots()
            .nth(self.screen_idx)
            .expect("Invalid screen")
    }

    fn expose(&self) {
        let inactive_attr = Attributes {
            font: pango::FontDescription::from_string("Envy Code R 27"),
            fg_color: Color::white(),
            bg_color: None,
            padding: Padding::new(10.0, 10.0, 5.0, 5.0),
        };
        let active_attr = inactive_attr.with_bg_color(Some(Color::blue()));

        let mut texts = Pager::new(active_attr, inactive_attr.clone()).compute_text();
        texts.extend(ActiveWindowTitle::new(inactive_attr.clone()).compute_text());
        texts.extend(Clock::new(inactive_attr.clone()).compute_text());

        self.render_text_blocks(texts);

        self.conn.flush();
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
            for layout in layouts.iter_mut() {
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
            for layout in stretched.iter_mut() {
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
}

fn handle_xcb_events(conn: &xcb::base::Connection, w: &Window) {
    // As we're edge triggered, we must completely drain all events before
    // returning to mio.
    // XXX Do we need to oneshot our EventedFd?
    while let Some(event) = conn.poll_for_event() {
        let r = event.response_type() & !0x80;
        match r {
            xcb::EXPOSE => w.expose(),
            _ => {}
        }
    }
}

fn main() {
    let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
    let conn = Rc::new(conn);

    let w = Window::new(conn.clone(), screen_idx as usize);

    let conn_fd = unsafe {
        xcb::ffi::base::xcb_get_file_descriptor(conn.get_raw_conn())
    };

    const TOKEN_XCB: Token = Token(0);
    let poll = Poll::new().unwrap();
    poll.register(&EventedFd(&conn_fd), TOKEN_XCB, Ready::readable(), PollOpt::edge()).unwrap();

    let mut events = Events::with_capacity(1024);
    loop {
        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            match event.token() {
                TOKEN_XCB => handle_xcb_events(&conn, &w),
                _ => unreachable!("Unhandled mio event"),
            }
        }
    }
}
