extern crate alsa;
extern crate cairo_sys;
extern crate cairo;
extern crate chrono;
#[macro_use]
extern crate error_chain;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate mio;
extern crate pango;
extern crate pangocairo;
extern crate regex;
extern crate tokio_core;
extern crate tokio_timer;
extern crate xcb_util;
extern crate xcb;

use std::rc::Rc;

use tokio_core::reactor::{Core, Handle};
use tokio_timer::Timer;

pub mod errors;
pub mod text;
pub mod widgets;
pub mod bar;

pub use bar::Position;
pub use errors::*;
pub use text::*;

use bar::Bar;
use widgets::Widget;


pub struct Hue {
    core: Core,
    timer: Rc<Timer>,
    bar: Bar,
    widgets: Vec<Box<Widget>>,
}

impl Hue {
    pub fn new(position: Position) -> Result<Hue> {
        Ok(Hue {
            core: Core::new().chain_err(|| "Could not create Tokio Core")?,
            timer: Rc::default(),
            bar: Bar::new(position)?,
            widgets: Vec::new(),
        })
    }

    fn handle(&self) -> Handle {
        self.core.handle()
    }

    fn timer(&self) -> Rc<Timer> {
        self.timer.clone()
    }

    pub fn add_widget<W>(&mut self, widget: W)
    where
        W: Widget + 'static,
    {
        self.widgets.push(Box::new(widget) as Box<Widget>);
    }

    pub fn run(mut self) -> Result<()> {
        let handle = self.handle();
        self.core
            .run(self.bar.run_event_loop(&handle, self.widgets)?)
    }
}


/// Convenience macro to get around lexical lifetime issue.
#[macro_export]
macro_rules! hue_add_widget {
    ($hue:ident, $widget:expr) => {
        let widget = $widget;
        $hue.add_widget(widget);
    }
}
