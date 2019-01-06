use std::f64;
use std::io;
use std::mem;
use std::os::unix::io::RawFd;
use std::rc::Rc;

use cairo::{self, XCBSurface};
use cairo_sys;
use futures::{future, Async, Future, Poll, Stream};
use itertools::Itertools;
use mio::event::Evented;
use mio::unix::EventedFd;
use mio::{self, PollOpt, Ready, Token};
use tokio_core::reactor::{Handle, PollEvented};
use xcb;
use xcb_util::ewmh;

use errors::*;
use text::{ComputedText, Text};
use widgets::{Widget, WidgetList};

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
            &mut get_root_visual_type(conn, screen).base as *mut xcb::ffi::xcb_visualtype_t
                as *mut cairo_sys::xcb_visualtype_t,
        )
    };
    let drawable = cairo::XCBDrawable(id);
    cairo::Surface::create(&cairo_conn, &drawable, &visual, width, height)
}

/// An enum specifying the position of the Cnx bar.
///
/// Passed to [`Cnx::new()`] when constructing a [`Cnx`] instance.
///
/// [`Cnx::new()`]: struct.Cnx.html#method.new
/// [`Cnx`]: struct.Cnx.html
///
/// # Examples
///
/// ```
/// # use cnx::{Cnx, Position};
/// let mut cnx = Cnx::new(Position::Top);
/// ```
#[derive(Clone, Debug)]
pub enum Position {
    /// Position the Cnx bar at the top of the screen.
    Top,
    /// Position the Cnx bar at the bottom of the screen.
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
    contents: Vec<Vec<ComputedText>>,
}

impl Bar {
    pub fn new(position: Position) -> Result<Bar> {
        let (conn, screen_idx) =
            xcb::Connection::connect(None).chain_err(|| "Failed to connect to X server")?;
        let screen_idx = screen_idx as usize;
        let id = conn.generate_id();

        // We don't actually care about how tall our initial window is - we'll resize
        // our window once we know how big it needs to be. However, it seems to need
        // to be bigger than 0px, or either Xcb/Cairo (or maybe QTile?) gets upset.
        let height = 1;

        let (width, surface) = {
            let screen = conn
                .get_setup()
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

            let surface = cairo_surface_for_xcb_window(
                &conn,
                &screen,
                id,
                i32::from(width),
                i32::from(height),
            );

            (width, surface)
        };

        let ewmh_conn = ewmh::Connection::connect(conn)
            .map_err(|(e, _)| e)
            .chain_err(|| "Failed to wrap xcb::Connection in ewmh::Connection")?;

        #[allow(blacklisted_name)]
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
            Position::Top => strut_partial.top = u32::from(self.height),
            Position::Bottom => strut_partial.bottom = u32::from(self.height),
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
                (xcb::CONFIG_WINDOW_Y as u16, u32::from(y)),
                (xcb::CONFIG_WINDOW_HEIGHT as u16, u32::from(self.height)),
                (xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE),
            ];
            xcb::configure_window(&self.conn, self.window_id, &values);
            self.map_window();
            self.surface
                .set_size(i32::from(self.width), i32::from(self.height));

            // Update EWMH properties - we might need to reserve more or less space.
            self.set_ewmh_properties();
        }

        Ok(())
    }

    fn update_widget_contents(&mut self, new_contents: Vec<Option<Vec<Text>>>) -> Result<bool> {
        // For each widget's texts:
        //  - If they're equal to the previous texts we had for it, do nothing.
        //  - If there are new texts or any non-stretch texts changed size, redraw
        //    the entire bar.
        //  - Otherwise, draw the texts that have changed since last time.
        let mut redraw_entire_bar = false;

        // Borrow these here, as otherwise our closures will try to borrow
        // self as both immutable/mutable.
        let surface = &self.surface;
        let contents = &mut self.contents;

        let it = new_contents
            .into_iter()
            .zip(contents.iter_mut())
            // We get a stream of updates from each widget, but not every
            // widget will have given us an update. Filter out those which are
            // None (no update).
            .filter_map(|(opt, old)| opt.map(|new| (new, old)))
            // Even if we have actually received an update, it may be identical
            // to the text it gave previously. (If that's the case, we can
            // avoid even calling .compute()).
            .filter(|&(ref new, ref old)| {
                let length_different = new.len() != old.len();
                let all_same = !length_different &&
                    new
                        .iter()
                        .zip(old.iter())
                        .all(|(n, o)| n == o);
                !all_same
            })
            // We finally have a list of the texts which have actually changed.
            // Call .compute() on each of the new texts so that we can get
            // layout information.
            .map(|(new, old)| {
                new.into_iter()
                    .map(|text| text.compute(surface))
                    .collect::<Result<Vec<ComputedText>>>()
                    .map(|computeds| (computeds, old))
            }).collect::<Result<Vec<_>>>()?;

        for (mut new_texts, old_texts) in it {
            // Redraw the entire bar if any of widget's non-stretch texts
            // have changed size, or if the number of texts for this widget
            // has changed. (Both of these would affect the size of other
            // stretch texts). Redraw the entire bar if any texts change
            // height. (It would be better to do this only if this changes
            // the height of the bar).
            let length_different = new_texts.len() != old_texts.len();
            redraw_entire_bar = redraw_entire_bar || length_different
                || new_texts.iter().zip(old_texts.iter()).any(|(new, old)| {
                    let not_stretch = !new.stretch && !old.stretch;
                    let diff_width = (new.width - old.width).abs().round() >= 1.0;
                    let diff_height = (new.height - old.height).abs().round() >= 1.0;
                    (not_stretch && diff_width) || diff_height
                });

            // Where possible, re-use the position of the widget's previous
            // texts. (If we re-draw the entire bar, it'll get updated anyway).
            // For stretch widgets, use the old width/height as well.
            for (new, old) in new_texts.iter_mut().zip(old_texts.iter()) {
                new.x = old.x;
                new.y = old.y;
                if !redraw_entire_bar && new.stretch {
                    new.width = old.width;
                    new.height = old.height;
                }
            }

            // If we're not redrawing the entire bar, then render this widget.
            // (It would actually be better if we could delay this render until
            // after we've processes all the other widgets, in case any of them
            // need to redraw the entire bar. However, that's more effort than
            // it's worth!)
            if !redraw_entire_bar {
                let changed = new_texts
                    .iter()
                    .zip(old_texts.iter())
                    .filter(|&(n, o)| n != o)
                    .map(|(n, _)| n);
                for text in changed {
                    trace!("Redrawing one");
                    text.render(surface)?;
                }
            }

            mem::swap(&mut new_texts, old_texts);
        }

        Ok(redraw_entire_bar)
    }

    fn redraw_entire_bar(&mut self) -> Result<()> {
        trace!("Redraw entire bar");

        // Calculate how much free space we have after laying out all the
        // non-stretch blocks. Split the remaining space (if any) between the
        // stretch blocks. If there isn't enough space for the non-stretch blocks
        // do nothing and allow it to overflow.
        // While we're at it, we also calculate how
        let screen_width = f64::from(
            self.screen()
                .chain_err(|| "Could not get screen width")?
                .width_in_pixels(),
        );
        let width_per_stretched = {
            let texts = self.contents.iter().flatten();
            let (stretched, non_stretched): (Vec<_>, Vec<_>) = texts.partition(|text| text.stretch);
            let width = non_stretched.iter().fold(0.0, |acc, text| {
                if text.stretch {
                    0.0
                } else {
                    acc + text.width
                }
            });
            let remaining_width = (screen_width - width).max(0.0);
            remaining_width / (stretched.len() as f64)
        };

        // Get the height of the biggest Text and set the bar to be that big.
        // TODO: Update all the Layouts so they all render that big too?
        let height = self
            .contents
            .iter()
            .flatten()
            .fold(f64::NEG_INFINITY, |acc, text| text.height.max(acc));
        if let Err(e) = self.update_bar_height(height as u16) {
            // Log and continue - the bar is hopefully still useful.
            error!("Failed to update bar height to {}: {}", height, e);
        }

        // Render each Text in turn. If it's a stretch block, override its width
        // with the width we've just computed. Regardless of whether it's a stretch
        // block, override its height - everything should be as big as the biggest item.
        let texts = self.contents.iter_mut().flatten();
        let mut x = 0.0;
        for text in texts {
            if text.stretch {
                text.width = width_per_stretched;
            }
            text.x = x;
            text.y = 0.0;
            text.render(&self.surface)?;
            x += text.width;
        }

        Ok(())
    }

    pub fn run_event_loop(
        mut self,
        handle: &Handle,
        widgets: Vec<Box<Widget>>,
    ) -> Result<Box<Future<Item = (), Error = Error>>> {
        self.contents = vec![Vec::new(); widgets.len()];

        enum Event {
            Xcb(<XcbEventStream as Stream>::Item),
            Widget(<WidgetList as Stream>::Item),
        };

        let events_stream = XcbEventStream::new(self.conn.clone(), handle)?.map(Event::Xcb);
        let widget_updates_stream = WidgetList::new(widgets)?.map(Event::Widget);
        let event_loop = events_stream.select(widget_updates_stream);

        let fut = event_loop.for_each(move |event| {
            let redraw_entire_bar = match event {
                Event::Widget(update) => match self.update_widget_contents(update) {
                    Ok(b) => b,
                    Err(e) => return future::err(e),
                },
                Event::Xcb(event) => event.response_type() & !0x80 == xcb::EXPOSE,
            };

            if redraw_entire_bar {
                if let Err(e) = self.redraw_entire_bar() {
                    error!("Error redrawing bar: {}", e);
                    return future::err(e);
                }
            }
            self.conn.flush();

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

pub(crate) struct XcbEventStream {
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
