extern crate cairo_sys;
extern crate cairo;
extern crate chrono;
extern crate futures;
extern crate mio;
extern crate pango;
extern crate pangocairo;
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
    let w = Bar::new();


    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let inactive_attr = Attributes {
        font: pango::FontDescription::from_string("Envy Code R 27"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(10.0, 10.0, 5.0, 5.0),
    };
    let active_attr = inactive_attr.with_bg_color(Some(Color::blue()));

    let widgets: Vec<Box<Widget>> =
        vec![Box::new(Pager::new(active_attr, inactive_attr.clone())) as Box<Widget>,
             Box::new(ActiveWindowTitle::new(inactive_attr.clone())) as Box<Widget>,
             Box::new(Clock::new(inactive_attr.clone())) as Box<Widget>];

    core.run(w.run_event_loop(&handle, widgets)).unwrap();
}
