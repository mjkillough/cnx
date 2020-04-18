use std::io;
use std::os::unix::io::RawFd;
use std::rc::Rc;

use failure::Error;
use futures::{Async, Poll, Stream};
use mio::event::Evented;
use mio::unix::EventedFd;
use mio::{PollOpt, Ready, Token};
use tokio_core::reactor::{Handle, PollEvented};
use xcb_util::ewmh;

use crate::Result;

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

