use anyhow::Result;

use cnx::text::*;
use cnx::widgets::*;
use cnx::{Cnx, Position};

fn main() -> Result<()> {
    let attr = Attributes {
        font: Font::new("Envy Code R 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let mut active_attr = attr.clone();
    active_attr.bg_color = Some(Color::blue());

    let mut cnx = Cnx::new(Position::Bottom);

    let sensors = Sensors::new(attr.clone(), vec!["Core 0", "Core 1"]);
    let battery = Battery::new(attr.clone(), Color::red());
    let volume = volume::Volume::new(attr.clone());
    cnx.add_widget(Pager::new(active_attr, attr.clone()));
    cnx.add_widget(ActiveWindowTitle::new(attr.clone()));
    cnx.add_widget(volume);
    cnx.add_widget(sensors);
    // cnx.add_widget(battery);
    cnx.add_widget(Clock::new(attr.clone()));
    cnx.run()?;

    Ok(())
}
