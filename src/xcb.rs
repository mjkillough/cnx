use std::io;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use anyhow::{anyhow, Context as _AnyhowContext, Result};
use mio::unix::EventedFd;
use mio::{Evented, Ready};
use tokio::io::PollEvented;
use tokio::stream::{self, Stream, StreamExt};
use xcb;
use xcb::xproto::{PropertyNotifyEvent, PROPERTY_NOTIFY};
use xcb_util::ewmh;

// A wrapper around `ewhm::Connection` that implements `mio::Evented`.
//
// This is just using `mio::EventedFd`. We have to have a custom wrapper
// because `mio::EventedFd` only borrows its fd and it's difficult to
// make it live long enough.
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
        token: mio::Token,
        interest: Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.fd()).deregister(poll)
    }
}

// A `Stream` of `xcb::GenericEvent` for the provided `xcb::Connection`.
pub struct XcbEventStream {
    conn: Rc<ewmh::Connection>,
    poll: PollEvented<XcbEvented>,
    would_block: bool,
}

impl XcbEventStream {
    pub fn new(conn: Rc<ewmh::Connection>) -> Result<XcbEventStream> {
        let evented = XcbEvented(conn.clone());
        let poll = PollEvented::new(evented)?;

        Ok(XcbEventStream {
            conn,
            poll,
            would_block: true,
        })
    }
}

impl Stream for XcbEventStream {
    type Item = xcb::GenericEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.would_block {
            match this.poll.poll_read_ready(cx, Ready::readable()) {
                Poll::Ready(Ok(_)) => this.would_block = false,
                Poll::Ready(Err(e)) => {
                    // Unsure when this would happen:
                    panic!("Error polling xcb::Connection: {}", e);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        match this.conn.poll_for_event() {
            Some(event) => Poll::Ready(Some(event)),
            None => {
                this.would_block = true;
                this.poll.clear_read_ready(cx, Ready::readable()).unwrap();
                Poll::Pending
            }
        }
    }
}

// A `Stream` that listens to `PROPERTY_CHANGE` notifications.
//
// By default it listens to `PROPERTY_CHANGE` notifications for the provided
// `properties` on the root window. The `ewhm::Connection` is returned so that
// the caller may listen to `PROPERTY_CHANGE` notifications on additional
// windows.
pub fn xcb_properties_stream(
    properties: &[&str],
) -> Result<(Rc<ewmh::Connection>, impl Stream<Item = ()>)> {
    let (xcb_conn, screen_idx) =
        xcb::Connection::connect(None).context("Failed to connect to X server")?;
    let root_window = xcb_conn
        .get_setup()
        .roots()
        .nth(screen_idx as usize)
        .ok_or_else(|| anyhow!("Invalid screen"))?
        .root();
    let ewmh_conn = ewmh::Connection::connect(xcb_conn)
        .map_err(|(e, _)| e)
        .context("Failed to wrap xcb::Connection in ewmh::Connection")?;
    let conn = Rc::new(ewmh_conn);

    let only_if_exists = true;
    let properties = properties
        .iter()
        .map(|property| -> Result<xcb::Atom> {
            let reply = xcb::intern_atom(&conn, only_if_exists, property).get_reply()?;
            Ok(reply.atom())
        })
        .collect::<Result<Vec<_>>>()
        .context("Failed to intern atoms")?;

    // Register for all PROPERTY_CHANGE events. We'll filter out the ones
    // that are interesting below.
    let attributes = [(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_PROPERTY_CHANGE)];
    xcb::change_window_attributes(&conn, root_window, &attributes);
    conn.flush();

    let xcb_stream = XcbEventStream::new(conn.clone())?;
    let stream = xcb_stream.filter_map(move |event| {
        if event.response_type() == PROPERTY_NOTIFY {
            let event: &PropertyNotifyEvent = unsafe { xcb::cast_event(&event) };
            if properties.iter().any(|p| *p == event.atom()) {
                // We don't actually care about the event, just that
                // it occurred.
                return Some(());
            }
        }
        None
    });

    // Pretend there was an initial property change to get the initial
    // contents of the widget, then allow our stream of XCB events to
    // call the callback for actual changes.
    let stream = stream::once(()).chain(stream);

    Ok((conn, stream))
}

