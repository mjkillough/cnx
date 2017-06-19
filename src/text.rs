use std::fmt;

use cairo::{Context, Surface};
use pango::{self, EllipsizeMode, FontDescription, Layout, LayoutExt};
use pangocairo::CairoContextExt;


#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    red: f64,
    green: f64,
    blue: f64,
}

macro_rules! color {
    ($name:ident, ($r:expr, $g:expr, $b:expr)) => {
        #[allow(dead_code)]
        pub fn $name() -> Color {
            Color {
                red: $r, green: $g, blue: $b,
            }
        }
    }
}

impl Color {
    color!(red, (1.0, 0.0, 0.0));
    color!(green, (0.0, 1.0, 0.0));
    color!(blue, (0.0, 0.0, 1.0));
    color!(white, (1.0, 1.0, 1.0));
    color!(black, (0.0, 0.0, 0.0));

    pub fn apply_to_context(&self, cr: &Context) {
        cr.set_source_rgb(self.red, self.green, self.blue);
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


#[derive(Clone, PartialEq)]
pub struct Font(FontDescription);

impl Font {
    pub fn new(name: &str) -> Font {
        Font(FontDescription::from_string(name))
    }
}


#[derive(Clone, PartialEq)]
pub struct Attributes {
    pub font: Font,
    pub fg_color: Color,
    pub bg_color: Option<Color>,
    pub padding: Padding,
}

impl fmt::Debug for Attributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Attributes {{ font: ?, fg_color: {:?}, bg_color: {:?}, padding {:?} }}",
            self.fg_color,
            self.bg_color,
            self.padding
        )
    }
}


#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub attr: Attributes,
    pub text: String,
    pub stretch: bool,
}

impl Text {
    pub fn compute(self, surface: &Surface) -> ComputedText {
        let (width, height) = {
            let context = Context::new(&surface);
            let layout = context.create_pango_layout();
            layout.set_text(&self.text, self.text.len() as i32);
            layout.set_font_description(Some(&self.attr.font.0));

            let padding = &self.attr.padding;
            let (text_width, text_height) = layout.get_pixel_size();
            let width = text_width as f64 + padding.left + padding.right;
            let height = text_height as f64 + padding.top + padding.bottom;
            (width, height)
        };

        ComputedText {
            attr: self.attr,
            text: self.text,
            stretch: self.stretch,
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
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
pub struct ComputedText {
    pub attr: Attributes,
    pub text: String,
    pub stretch: bool,

    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ComputedText {
    pub fn render(&self, surface: &Surface) {
        let context = Context::new(&surface);
        let layout = context.create_pango_layout();
        layout.set_text(&self.text, self.text.len() as i32);
        layout.set_font_description(Some(&self.attr.font.0));

        context.translate(self.x, self.y);

        // Set the width/height on the Pango layout so that it word-wraps/ellipises.
        let padding = &self.attr.padding;
        let text_width = self.width - padding.left - padding.right;
        let text_height = self.height - padding.top - padding.bottom;
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_width(text_width as i32 * pango::SCALE);
        layout.set_height(text_height as i32 * pango::SCALE);

        let bg_color = &self.attr.bg_color.clone().unwrap_or_else(|| Color::black());
        bg_color.apply_to_context(&context);
        // FIXME: The use of `height` isnt' right here: we want to do the
        // full height of the bar, not the full height of the text. It
        // would be useful if we could do Surface.get_height(), but that
        // doesn't seem to be available in cairo-rs for some reason?
        context.rectangle(0.0, 0.0, self.width, self.height);
        context.fill();

        self.attr.fg_color.apply_to_context(&context);
        context.translate(padding.left, padding.top);
        context.show_pango_layout(&layout);
    }
}
