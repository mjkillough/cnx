#[macro_use]
extern crate cnx;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate log;

use std::env;

use env_logger::LogBuilder;
use log::LogLevelFilter;

use cnx::*;
use cnx::text::*;
use cnx::widgets::*;


// This will not be needed in error-chain 0.11:
#[allow(unknown_lints, unused_doc_comment)]
mod errors {
    error_chain! {
        links {
            Cnx(::cnx::errors::Error, ::cnx::errors::ErrorKind);
        }
        foreign_links {
            SetLogger(::log::SetLoggerError);
        }
    }
}


fn init_log() -> errors::Result<()> {
    let mut builder = LogBuilder::new();
    builder.filter(Some("cnx"), LogLevelFilter::Trace);
    if let Ok(rust_log) = env::var("RUST_LOG") {
       builder.parse(&rust_log);
    }
    Ok(builder.init()?)
}

fn run() -> errors::Result<()> {
    init_log()?;

    let attr = Attributes {
        font: Font::new("SourceCodePro 21"),
        fg_color: Color::white(),
        bg_color: None,
        padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    };
    let mut active_attr = attr.clone();
    active_attr.bg_color = Some(Color::blue());

    let mut cnx = Cnx::new(Position::Top)?;

    cnx_add_widget!(cnx, Pager::new(&cnx, active_attr, attr.clone()));
    cnx_add_widget!(cnx, ActiveWindowTitle::new(&cnx, attr.clone()));
    cnx_add_widget!(
        cnx,
        Sensors::new(&cnx, attr.clone(), vec!["Core 0", "Core 1"])
    );
    #[cfg(feature = "volume-widget")]
    cnx_add_widget!(cnx, Volume::new(&cnx, attr.clone()));
    cnx_add_widget!(cnx, Battery::new(&cnx, attr.clone(), Color::red()));
    cnx_add_widget!(cnx, Clock::new(&cnx, attr.clone()));

    Ok(cnx.run()?)
}

quick_main!(run);
