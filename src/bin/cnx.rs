#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate cnx;

use cnx::*;
use cnx::widgets::*;


fn run() -> Result<()> {
    let attr = Attributes {
        font: Font::new("SourceCodePro 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let mut active_attr = attr.clone();
    active_attr.bg_color = Some(Color::blue());

    let mut cnx = Cnx::new(Position::Bottom)?;

    cnx_add_widget!(cnx, Pager::new(&cnx, active_attr, attr.clone()));
    cnx_add_widget!(cnx, ActiveWindowTitle::new(&cnx, attr.clone()));
    cnx_add_widget!(
        cnx,
        Sensors::new(&cnx, attr.clone(), vec!["Core 0", "Core 1"])
    );
    cnx_add_widget!(cnx, Volume::new(&cnx, attr.clone()));
    cnx_add_widget!(cnx, Battery::new(&cnx, attr.clone()));
    cnx_add_widget!(cnx, Clock::new(&cnx, attr.clone()));

    cnx.run()
}

quick_main!(run);
