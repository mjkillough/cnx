//! Types to represent text to be displayed by widgets.
//!
//! This module is light on documentation. See the existing widget
//! implementations for inspiration.

use anyhow::Result;
use cairo::{Context, Surface};
use colors_transform::{Color as ColorTransform, Rgb};
use pango::{EllipsizeMode, FontDescription};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    red: f64,
    green: f64,
    blue: f64,
}

macro_rules! color {
    ($name:ident,($r:expr, $g:expr, $b:expr)) => {
        #[allow(dead_code)]
        pub fn $name() -> Color {
            Color {
                red: $r,
                green: $g,
                blue: $b,
            }
        }
    };
}

impl Color {
    color!(red, (1.0, 0.0, 0.0));
    color!(green, (0.0, 1.0, 0.0));
    color!(blue, (0.0, 0.0, 1.0));
    color!(white, (1.0, 1.0, 1.0));
    color!(black, (0.0, 0.0, 0.0));
    color!(yellow, (1.0, 1.0, 0.0));

    pub fn apply_to_context(&self, cr: &Context) {
        cr.set_source_rgb(self.red, self.green, self.blue);
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            red: r as f64 / 255.0,
            green: g as f64 / 255.0,
            blue: b as f64 / 255.0,
        }
    }

    /// Parse string as hex color
    /// # Example
    /// ```
    /// use cnx::text::Color;
    ///
    /// assert_eq!(Color::from_hex("#1e1e2e"), Color::from_rgb(30, 30, 46));
    /// assert_eq!(Color::from_hex("not hex"), Color::from_rgb(0, 0, 0));
    /// ```
    pub fn from_hex(hex: &str) -> Self {
        let rgb = match Rgb::from_hex_str(hex) {
            Ok(rgb) => rgb,
            Err(_) => Rgb::from(0.0, 0.0, 0.0),
        };

        Self {
            red: rgb.get_red() as f64 / 255.0,
            green: rgb.get_green() as f64 / 255.0,
            blue: rgb.get_blue() as f64 / 255.0,
        }
    }

    pub fn to_hex(&self) -> String {
        let r = if self.red >= 1.0 {
            255
        } else {
            (self.red * 255.0) as i32
        };
        let g = if self.green >= 1.0 {
            255
        } else {
            (self.green * 255.0) as i32
        };
        let b = if self.blue >= 1.0 {
            255
        } else {
            (self.blue * 255.0) as i32
        };
        format!("#{:0width$X}{:0width$X}{:0width$X}", r, g, b, width = 2)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Padding {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Padding {
    pub fn new(left: f64, right: f64, top: f64, bottom: f64) -> Padding {
        Padding {
            left,
            right,
            top,
            bottom,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Font(FontDescription);

impl Font {
    pub fn new(name: &str) -> Font {
        Font(FontDescription::from_string(name))
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attributes {
    pub font: Font,
    pub fg_color: Color,
    pub bg_color: Option<Color>,
    pub padding: Padding,
}

pub struct PagerAttributes {
    /// Active attributes are applied to the currently active workspace
    pub active_attr: Attributes,
    /// Inactive attributes are applied to workspaces that are not active and contain no windows
    pub inactive_attr: Attributes,
    /// Non empty attributes are applied to workspaces that are not active and contain windows
    pub non_empty_attr: Attributes,
}

fn create_pango_layout(cairo_context: &cairo::Context) -> pango::Layout {
    pangocairo::functions::create_layout(cairo_context)
}

fn show_pango_layout(cairo_context: &cairo::Context, layout: &pango::Layout) {
    pangocairo::functions::show_layout(cairo_context, layout);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub attr: Attributes,
    pub text: String,
    pub stretch: bool,
    pub markup: bool,
}

impl Text {
    pub(crate) fn compute(self, surface: &Surface) -> Result<ComputedText> {
        let (width, height) = {
            let context = Context::new(surface)?;
            let layout = create_pango_layout(&context);
            if self.markup {
                layout.set_markup(&self.text);
            } else {
                layout.set_text(&self.text);
            }
            layout.set_font_description(Some(&self.attr.font.0));

            let padding = &self.attr.padding;
            let (text_width, text_height) = layout.pixel_size();
            let width = f64::from(text_width) + padding.left + padding.right;
            let height = f64::from(text_height) + padding.top + padding.bottom;
            (width, height)
        };

        Ok(ComputedText {
            attr: self.attr,
            text: self.text,
            stretch: self.stretch,
            x: 0.0,
            y: 0.0,
            width,
            height,
            markup: self.markup,
        })
    }
}

// This impl allows us to see whether a widget's text has changed without
// having to call the (relatively) expensive .compute().
impl PartialEq<ComputedText> for Text {
    fn eq(&self, other: &ComputedText) -> bool {
        self.attr == other.attr && self.text == other.text && self.stretch == other.stretch
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedText {
    pub attr: Attributes,
    pub text: String,
    pub stretch: bool,

    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub markup: bool,
}

impl ComputedText {
    pub fn render(&self, surface: &Surface) -> Result<()> {
        let context = Context::new(surface)?;
        let layout = create_pango_layout(&context);
        if self.markup {
            layout.set_markup(&self.text);
        } else {
            layout.set_text(&self.text);
        }
        layout.set_font_description(Some(&self.attr.font.0));

        context.translate(self.x, self.y);

        // Set the width/height on the Pango layout so that it word-wraps/ellipises.
        let padding = &self.attr.padding;
        let text_width = self.width - padding.left - padding.right;
        let text_height = self.height - padding.top - padding.bottom;
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_width(text_width as i32 * pango::SCALE);
        layout.set_height(text_height as i32 * pango::SCALE);

        let bg_color = &self.attr.bg_color.clone().unwrap_or_else(Color::black);
        bg_color.apply_to_context(&context);
        // FIXME: The use of `height` isnt' right here: we want to do the
        // full height of the bar, not the full height of the text. It
        // would be useful if we could do Surface.get_height(), but that
        // doesn't seem to be available in cairo-rs for some reason?
        context.rectangle(0.0, 0.0, self.width, self.height);
        context.fill()?;

        self.attr.fg_color.apply_to_context(&context);
        context.translate(padding.left, padding.top);
        show_pango_layout(&context, &layout);

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThresholdValue {
    pub threshold: u8,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Threshold {
    pub low: ThresholdValue,
    pub normal: ThresholdValue,
    pub high: ThresholdValue,
}

impl Default for Threshold {
    fn default() -> Self {
        Threshold {
            low: ThresholdValue {
                threshold: 40,
                color: Color::red(),
            },
            normal: ThresholdValue {
                threshold: 60,
                color: Color::yellow(),
            },
            high: ThresholdValue {
                threshold: 100,
                color: Color::green(),
            },
        }
    }
}
