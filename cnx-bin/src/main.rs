use anyhow::Result;
use byte_unit::ByteUnit;
use cnx::text::*;
use cnx::widgets::*;
use cnx::{Cnx, Position};
use cnx_contrib::widgets::battery::*;
use cnx_contrib::widgets::disk_usage::*;
use cnx_contrib::widgets::*;
use weathernoaa::weather::WeatherInfo;

fn pango_markup_render(color: Color, start_text: String, text: String) -> String {
    format!(
            "<span foreground=\"#808080\">[</span>{} <span foreground=\"{}\">{}</span><span foreground=\"#808080\">]</span>",
        start_text, color.to_hex(), text
        )
}

fn pango_markup_single_render(color: Color, start_text: String) -> String {
    format!(
            "<span foreground=\"#808080\">[</span>{}<span foreground=\"{}\"></span><span foreground=\"#808080\">]</span>",
        start_text, color.to_hex()
        )
}

fn weather_sky_condition(condition: String) -> &'static str {
    match &condition[..] {
        "clear" => "🌣",
        "sunny" => "🌣",
        "mostly clear" => "🌤",
        "mostly sunny" => "🌤",
        "partly sunny" => "⛅",
        "fair" => "🌑",
        "cloudy" => "☁",
        "overcast" => "☁",
        "partly cloudy" => "⛅",
        "mostly cloudy" => "🌧",
        "considerable cloudines" => "☔",
        _ => "🌑",
    }
}

fn main() -> Result<()> {
    let attr = Attributes {
        font: Font::new("Ubuntu Mono Bold 14"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(0.0, 0.0, 0.0, 0.0),
    };

    let mut cnx = Cnx::new(Position::Bottom);

    // let sensors = Sensors::new(attr.clone(), vec!["Core 0", "Core 1"]);
    let battery_render = Box::new(|battery_info: BatteryInfo| {
        let percentage = battery_info.capacity;

        let default_text = format!("🔋{percentage:.0}%",);
        pango_markup_single_render(Color::white(), default_text)
    });

    let battery = Battery::new(attr.clone(), Color::red(), None, Some(battery_render));
    let render = Box::new(|load| {
        let mut color = Color::yellow().to_hex();
        if load < 5 {
            color = Color::green().to_hex();
        }
        if load > 50 {
            color = Color::red().to_hex();
        }
        format!(
            "<span foreground=\"#808080\">[</span>Cpu: <span foreground=\"{color}\">{load}%</span><span foreground=\"#808080\">]</span>"
        )
    });
    let cpu = cpu::Cpu::new(attr.clone(), Some(render))?;

    let volume = volume::Volume::new(attr.clone());

    let default_threshold = Threshold::default();

    let wireless =
        wireless::Wireless::new(attr.clone(), "wlp2s0".to_owned(), Some(default_threshold));

    let disk_render = Box::new(|disk_info: DiskInfo| {
        let used = disk_info.used.get_adjusted_unit(ByteUnit::GiB).format(0);
        let total = disk_info.total.get_adjusted_unit(ByteUnit::GiB).format(0);
        let disk_text = format!("🏠 {used}/{total}");
        pango_markup_single_render(Color::white(), disk_text)
    });

    let disk_usage = disk_usage::DiskUsage::new(attr.clone(), "/home".into(), Some(disk_render));

    let weather_render = Box::new(|weather: WeatherInfo| {
        let sky_condition = weather_sky_condition(weather.sky_condition);
        let weather_text = format!("BLR: {sky_condition} :");
        let weather_temp = format!(" {}°C", weather.temperature.celsius);
        pango_markup_render(Color::white(), weather_text, weather_temp)
    });

    let weather = weather::Weather::new(attr.clone(), "VOBL".into(), Some(weather_render));

    let active_attr = Attributes {
        font: Font::new("Ubuntu Mono Bold 14"),
        fg_color: Color::white(),
        bg_color: Some(Color::blue()),
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let inactive_attr = Attributes {
        bg_color: None,
        ..active_attr.clone()
    };
    let non_empty_attr = Attributes {
        fg_color: Color::blue(),
        ..inactive_attr.clone()
    };
    let pager_attrs = PagerAttributes {
        active_attr,
        inactive_attr,
        non_empty_attr,
    };
    let pager = Pager::new(pager_attrs);

    cnx.add_widget(pager);
    cnx.add_widget(ActiveWindowTitle::new(attr.clone()));
    cnx.add_widget(cpu);
    cnx.add_widget(weather);
    cnx.add_widget(disk_usage);
    cnx.add_widget(wireless);
    cnx.add_widget(volume);

    // cnx.add_widget(sensors);
    cnx.add_widget(battery);
    let time_template = Some("<span foreground=\"#808080\">[</span>%d-%m-%Y %a %I:%M %p<span foreground=\"#808080\">]</span>".into());
    cnx.add_widget(Clock::new(attr, time_template));
    cnx.run()?;

    Ok(())
}
