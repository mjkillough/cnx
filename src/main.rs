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

mod errors;
use errors::*;
mod text;
use text::*;
mod widgets;
use widgets::*;
mod bar;
use bar::*;


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


fn run() -> Result<()> {
    let attr = Attributes {
        font: pango::FontDescription::from_string("SourceCodePro 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let active_attr = attr.with_bg_color(Some(Color::blue()));

    let mut hue = Hue::new(Position::Bottom)?;

    // Yucky macro to get around the fact that hue.add_widget(... &hue)
    // complains about the second immutable borrow. We should be able to get
    // rid of this if we implement a clever builder pattern for widgets which
    // Hue can use.
    macro_rules! add_widget {
        ($hue:ident, $widget:expr) => {
            let widget = $widget;
            $hue.add_widget(widget);
        }
    }
    add_widget!(hue, Pager::new(&hue, active_attr, attr.clone()));
    add_widget!(hue, ActiveWindowTitle::new(&hue, attr.clone()));
    add_widget!(
        hue,
        Sensors::new(&hue, attr.clone(), vec!["Core 0", "Core 1"])
    );
    add_widget!(hue, Volume::new(&hue, attr.clone()));
    add_widget!(hue, Battery::new(&hue, attr.clone()));
    add_widget!(hue, Clock::new(&hue, attr.clone()));

    hue.run()
}

quick_main!(run);
