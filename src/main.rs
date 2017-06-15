extern crate alsa;
extern crate cairo_sys;
extern crate cairo;
extern crate chrono;
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

use tokio_core::reactor::Core;

mod text;
use text::*;
mod widgets;
use widgets::*;
mod bar;
use bar::*;


fn main() {
    let w = Bar::new(Position::Bottom);


    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let inactive_attr = Attributes {
        font: pango::FontDescription::from_string("SourceCodePro 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let active_attr = inactive_attr.with_bg_color(Some(Color::blue()));

    let sensors = vec!["Core 0".to_owned(), "Core 1".to_owned()];

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
            Box::new(Sensors::new(inactive_attr.clone(), sensors)) as Box<Widget>,
            Box::new(Volume::new(handle.clone(), inactive_attr.clone())) as Box<Widget>,
            Box::new(Battery::new(inactive_attr.clone())) as Box<Widget>,
            Box::new(Clock::new(inactive_attr.clone())) as Box<Widget>,
        ];

    core.run(w.run_event_loop(&handle, widgets)).unwrap();
}
