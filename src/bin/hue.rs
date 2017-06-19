#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate hue;

use hue::*;
use hue::widgets::*;


fn run() -> Result<()> {
    let attr = Attributes {
        font: Font::new("SourceCodePro 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let mut active_attr = attr.clone();
    active_attr.bg_color = Some(Color::blue());

    let mut hue = Hue::new(Position::Bottom)?;

    hue_add_widget!(hue, Pager::new(&hue, active_attr, attr.clone()));
    hue_add_widget!(hue, ActiveWindowTitle::new(&hue, attr.clone()));
    hue_add_widget!(
        hue,
        Sensors::new(&hue, attr.clone(), vec!["Core 0", "Core 1"])
    );
    hue_add_widget!(hue, Volume::new(&hue, attr.clone()));
    hue_add_widget!(hue, Battery::new(&hue, attr.clone()));
    hue_add_widget!(hue, Clock::new(&hue, attr.clone()));

    hue.run()
}

quick_main!(run);
