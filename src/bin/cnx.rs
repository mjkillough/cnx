#![deny(warnings)]

use std::env;

use env_logger::Builder;
use log::LevelFilter;

use cnx::text::*;
use cnx::widgets::*;
use cnx::*;

fn init_log() -> Result<()> {
    let mut builder = Builder::new();
    builder.filter(Some("cnx"), LevelFilter::Trace);
    if let Ok(rust_log) = env::var("RUST_LOG") {
        builder.parse_filters(&rust_log);
    }
    builder.try_init()?;
    Ok(())
}

fn main() -> Result<()> {
    init_log()?;

    let attr = Attributes {
        font: Font::new("Envy Code R 22"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let mut active_attr = attr.clone();
    active_attr.bg_color = Some(Color::blue());

    let mut cnx = Cnx::new(Position::Bottom)?;

    cnx_add_widget!(cnx, Pager::new(&cnx, active_attr, attr.clone()));
    cnx_add_widget!(cnx, ActiveWindowTitle::new(&cnx, attr.clone()));
    // cnx_add_widget!(
    //    cnx,
    //    Sensors::new(&cnx, attr.clone(), vec!["Core 0", "Core 1"])
    // );
    #[cfg(feature = "volume-widget")]
    cnx_add_widget!(cnx, Volume::new(&cnx, attr.clone()));
    cnx_add_widget!(cnx, Battery::new(&cnx, attr.clone(), Color::red()));
    cnx_add_widget!(cnx, Clock::new(&cnx, attr.clone()));

    cnx.run()?;

    Ok(())
}
