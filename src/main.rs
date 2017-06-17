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

use tokio_core::reactor::Core;
use tokio_timer::Timer;

mod errors;
use errors::*;
mod text;
use text::*;
mod widgets;
use widgets::*;
mod bar;
use bar::*;


fn run() -> Result<()> {
    let mut core = Core::new().chain_err(|| "Could not create Tokio Core")?;
    let handle = core.handle();

    let inactive_attr = Attributes {
        font: pango::FontDescription::from_string("SourceCodePro 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let active_attr = inactive_attr.with_bg_color(Some(Color::blue()));

    let sensors = vec!["Core 0", "Core 1"];
    let timer = Rc::new(Timer::default());
    let widgets: Vec<Box<Widget>> =
        vec![
            Box::new(Pager::new(
                handle.clone(),
                active_attr,
                inactive_attr.clone(),
            )) as Box<Widget>,
            Box::new(ActiveWindowTitle::new(
                handle.clone(),
                inactive_attr.clone(),
            )) as Box<Widget>,
            Box::new(Sensors::new(timer.clone(), inactive_attr.clone(), sensors)) as Box<Widget>,
            Box::new(Volume::new(handle.clone(), inactive_attr.clone())) as Box<Widget>,
            Box::new(Battery::new(timer.clone(), inactive_attr.clone())) as Box<Widget>,
            Box::new(Clock::new(timer.clone(), inactive_attr.clone())) as Box<Widget>,
        ];

    let bar = Bar::new(Position::Top)?;
    core.run(bar.run_event_loop(&handle, widgets)?)?;

    Ok(())
}

quick_main!(run);
