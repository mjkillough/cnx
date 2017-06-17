use std::f64;
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
use xcb_util::ewmh;

use errors::*;
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
fn cairo_surface_for_xcb_window(
    conn: &xcb::Connection,
    screen: &xcb::Screen,
    id: u32,
    width: i32,
    height: i32,
) -> cairo::Surface {
    let cairo_conn = unsafe {
        cairo::XCBConnection::from_raw_none(conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t)
    };
    let visual = unsafe {
        cairo::XCBVisualType::from_raw_none(
            &mut get_root_visual_type(conn, screen).base as *mut xcb::ffi::xcb_visualtype_t as
                *mut cairo_sys::xcb_visualtype_t,
        )
    };
    let drawable = cairo::XCBDrawable(id);
    cairo::Surface::create(&cairo_conn, &drawable, &visual, width, height)
}


#[derive(Clone, Debug)]
pub enum Position {
    Top,
    Bottom,
}


pub struct Bar {
    conn: Rc<ewmh::Connection>,
    window_id: u32,
    screen_idx: usize,
    surface: cairo::Surface,
    width: u16,
    height: u16,
    position: Position,
    contents: Vec<Vec<Text>>,
}

impl Bar {
    pub fn new(position: Position) -> Result<Bar> {
        let (conn, screen_idx) = xcb::Connection::connect(None)
            .chain_err(|| "Failed to connect to X server")?;
        let screen_idx = screen_idx as usize;
        let id = conn.generate_id();

        // We don't actually care about how tall our initial window is - we'll resize
        // our window once we know how big it needs to be. However, it seems to need
        // to be bigger than 0px, or either Xcb/Cairo (or maybe QTile?) gets upset.
        let height = 1;

        let (width, surface) = {
            let screen = conn.get_setup()
                .roots()
                .nth(screen_idx)
                .ok_or("Invalid screen")?;
            let values = [
                (xcb::CW_BACK_PIXEL, screen.black_pixel()),
                (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE),
            ];

            let width = screen.width_in_pixels();

            xcb::create_window(
                &conn,
                xcb::COPY_FROM_PARENT as u8,
                id,
                screen.root(),
                0,
                0,
                width,
                height,
                0,
                xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
                screen.root_visual(),
                &values,
            );

            let surface =
                cairo_surface_for_xcb_window(&conn, &screen, id, width as i32, height as i32);

            (width, surface)
        };

        let ewmh_conn = ewmh::Connection::connect(conn)
            .map_err(|(e, _)| e)
            .chain_err(|| "Failed to wrap xcb::Connection in ewmh::Connection")?;

        let bar = Bar {
            conn: Rc::new(ewmh_conn),
            window_id: id,
            screen_idx,
            surface,
            width,
            height,
            position,
            contents: Vec::new(),
        };
        bar.set_ewmh_properties();
        // XXX We can't map the window until we've updated the window size, or nothing
        // gets rendered. I can't tell if this is something we're doing, something Cairo
        // is doing or something QTile is doing. This'll do for now and we'll see what
        // it is like with Lanta!
        // bar.map_window();
        bar.flush();
        Ok(bar)
    }

    fn flush(&self) {
        self.conn.flush();
    }

    fn map_window(&self) {
        xcb::map_window(&self.conn, self.window_id);
    }

    fn set_ewmh_properties(&self) {
        ewmh::set_wm_window_type(
            &self.conn,
            self.window_id,
            &[self.conn.WM_WINDOW_TYPE_DOCK()],
        );

        // TODO: Update _WM_STRUT_PARTIAL if the height/position of the bar changes?
        let mut strut_partial = ewmh::StrutPartial {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
            left_start_y: 0,
            left_end_y: 0,
            right_start_y: 0,
            right_end_y: 0,
            top_start_x: 0,
            top_end_x: 0,
            bottom_start_x: 0,
            bottom_end_x: 0,
        };
        match self.position {
            Position::Top => strut_partial.top = self.height as u32,
            Position::Bottom => strut_partial.bottom = self.height as u32,
        }
        ewmh::set_wm_strut_partial(&self.conn, self.window_id, strut_partial);
    }

    fn screen(&self) -> Result<xcb::Screen> {
        self.conn
            .get_setup()
            .roots()
            .nth(self.screen_idx)
            .ok_or_else(|| "Invalid screen".into())
    }

    fn update_bar_height(&mut self, height: u16) -> Result<()> {
        if self.height != height {
            self.height = height;

            // If we're at the bottom of the screen, we'll need to update the
            // position of the window.
            let y = match self.position {
                Position::Top => 0,
                Position::Bottom => self.screen()?.height_in_pixels() - self.height,
            };

            // Update the height/position of the XCB window and the height of the Cairo surface.
            let values = [
                (xcb::CONFIG_WINDOW_Y as u16, y as u32),
                (xcb::CONFIG_WINDOW_HEIGHT as u16, self.height as u32),
                (xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE),
            ];
            xcb::configure_window(&self.conn, self.window_id, &values);
            self.map_window();
            self.surface.set_size(self.width as i32, self.height as i32);

            // Update EWMH properties - we might need to reserve more or less space.
            self.set_ewmh_properties();
        }

        Ok(())
    }

    fn update_contents(&mut self, new_contents: Vec<Option<Vec<Text>>>) {
        for (i, opt) in new_contents.into_iter().enumerate() {
            if let Some(new) = opt {
                self.contents[i] = new;
            }
        }
    }

    fn render_text_blocks(&mut self, texts: Vec<Text>) -> Result<()> {
        // Calculate the width/height of each text block.
        let geometries: Vec<_> = texts
            .iter()
            .map(|t| t.compute_width_and_height(&self.surface))
            .collect();

        // Calculate how much free space we have after laying out all the
        // non-stretch blocks. Split the remaining space (if any) between the
        // stretch blocks. If there isn't enough space for the non-stretch blocks
        // do nothing and allow it to overflow.
        // While we're at it, we also calculate how
        let screen_width = self.screen()
            .chain_err(|| "Could not get screen width")?
            .width_in_pixels() as f64;
        let width_per_stretched = {
            let (stretched, non_stretched): (Vec<_>, Vec<_>) = texts
                .iter()
                .zip(geometries.clone())
                .partition(|&(t, _)| t.stretch);
            let width = non_stretched
                .iter()
                .fold(0.0, |acc, &(text, (width, _))| if text.stretch {
                    0.0
                } else {
                    acc + width
                });
            let remaining_width = (screen_width - width).max(0.0);
            remaining_width / (stretched.len() as f64)
        };

        // Get the height of the biggest Text and set the bar to be that big.
        // TODO: Update all the Layouts so they all render that big too?
        let height = geometries.iter().cloned().fold(
            f64::NEG_INFINITY,
            |acc, (_, h)| h.max(acc),
        );
        if let Err(_) = self.update_bar_height(height as u16) {
            // Log and continue - the bar is hopefully still useful.
            // TODO: Add log dependency.
            // error!("Failed to update bar height to {}: {}", height, e);
        }

        // Render each Text in turn. If it's a stretch block, override its width
        // with the width we've just computed. Regardless of whether it's a stretch
        // block, override its height - everything should be as big as the biggest item.
        let mut x = 0.0;
        for text in &texts {
            let width = if text.stretch {
                Some(width_per_stretched)
            } else {
                None
            };
            let (actual_width, _) = text.render(&self.surface, x, 0.0, width, Some(height));
            x += actual_width;
        }

        Ok(())
    }

    fn expose(&mut self) -> Result<()> {
        // Clear to black before re-painting.
        let context = Context::new(&self.surface);
        Color::black().apply_to_context(&context);
        context.paint();

        let flattened_contents = self.contents.clone().into_iter().flat_map(|v| v).collect();

        self.render_text_blocks(flattened_contents)?;
        self.conn.flush();

        Ok(())
    }

    pub fn run_event_loop(
        mut self,
        handle: &Handle,
        widgets: Vec<Box<Widget>>,
    ) -> Result<Box<Future<Item = (), Error = Error>>> {
        self.contents = vec![Vec::new(); widgets.len()];

        let events_stream = XcbEventStream::new(self.conn.clone(), handle)?;
        let widget_updates_stream = WidgetList::new(widgets)?;

        let event_loop = events_stream.merge(widget_updates_stream);
        let fut = event_loop.for_each(move |item| {
            let (xcb_event, widget_update) = match item {
                MergedItem::First(e) => (Some(e), None),
                MergedItem::Second(u) => (None, Some(u)),
                MergedItem::Both(e, u) => (Some(e), Some(u)),
            };

            let mut need_expose = false;
            if let Some(update) = widget_update {
                self.update_contents(update);
                need_expose = true;
            }
            if let Some(event) = xcb_event {
                if let xcb::EXPOSE = event.response_type() & !0x80 {
                    need_expose = true;
                }
            }
            if need_expose {
                if let Err(e) = self.expose() {
                    return future::err(e);
                }
            }

            future::ok(())
        });

        Ok(Box::new(fut))
    }
}


struct XcbEvented(Rc<ewmh::Connection>);

impl XcbEvented {
    fn fd(&self) -> RawFd {
        let conn: &xcb::Connection = &self.0;
        unsafe { xcb::ffi::base::xcb_get_file_descriptor(conn.get_raw_conn()) }
    }
}

impl Evented for XcbEvented {
    fn register(
        &self,
        poll: &mio::Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.fd()).deregister(poll)
    }
}


pub struct XcbEventStream {
    conn: Rc<ewmh::Connection>,
    poll: PollEvented<XcbEvented>,
    would_block: bool,
}

impl XcbEventStream {
    pub fn new(conn: Rc<ewmh::Connection>, handle: &Handle) -> Result<XcbEventStream> {
        let evented = XcbEvented(conn.clone());
        Ok(XcbEventStream {
            conn,
            poll: PollEvented::new(evented, handle)?,
            would_block: true,
        })
    }
}

impl Stream for XcbEventStream {
    type Item = xcb::GenericEvent;
    type Error = Error;

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
