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

    let sensors = vec!["hw.sensors.asmc0.fan0", "hw.sensors.cpu0.temp0"];

    cnx.add_widget(Pager::new(active_attr, attr.clone()));
    cnx.add_widget(ActiveWindowTitle::new(attr.clone()));
    cnx.add_widget(Sensors::new(attr.clone(), sensors));
    #[cfg(feature = "sioctl-volume")]
    cnx.add_widget(Volume::new(attr.clone()));
    cnx.add_widget(Battery::new(attr.clone(), Color::red()));
    cnx.add_widget(Clock::new(attr.clone()));

    cnx.run()?;

    Ok(())
}
