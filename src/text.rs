use cairo::{Context, Surface};
use pango::{FontDescription, Layout, LayoutExt};
use pangocairo::CairoContextExt;


#[derive(Clone, Debug)]
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


#[derive(Clone, Debug)]
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

    pub fn none() -> Padding {
        Padding {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }

    pub fn uniform(value: f64) -> Padding {
        Padding {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}


#[derive(Clone)]
pub struct Attributes {
    pub font: FontDescription,
    pub fg_color: Color,
    pub bg_color: Option<Color>,
    pub padding: Padding,
}

impl Attributes {
    pub fn with_font(&self, font: FontDescription) -> Attributes {
        let mut new = self.clone();
        new.font = font;
        new
    }

    pub fn with_fg_color(&self, fg_color: Color) -> Attributes {
        let mut new = self.clone();
        new.fg_color = fg_color;
        new
    }

    pub fn with_bg_color(&self, bg_color: Option<Color>) -> Attributes {
        let mut new = self.clone();
        new.bg_color = bg_color;
        new
    }

    pub fn with_padding(&self, padding: Padding) -> Attributes {
        let mut new = self.clone();
        new.padding = padding;
        new
    }
}


#[derive(Clone)]
pub struct Text {
    pub attr: Attributes,
    pub text: String,
    pub stretch: bool,
}

impl Text {
    pub fn layout(self, surface: &Surface) -> TextLayout {
        let context = Context::new(&surface);

        let layout = context.create_pango_layout();
        layout.set_text(&self.text, self.text.len() as i32);
        layout.set_font_description(Some(&self.attr.font));

        TextLayout {
            attr: self.attr.clone(),
            stretch: self.stretch,
            width: None,
            height: None,
            context: context,
            layout: layout,
        }
    }
}


pub struct TextLayout {
    attr: Attributes,
    stretch: bool,
    width: Option<f64>,
    height: Option<f64>,
    context: Context,
    layout: Layout,
}

impl TextLayout {
    pub fn width(&self) -> f64 {
        self.width
            .unwrap_or_else(|| {
                                let text_width = self.layout.get_pixel_size().0 as f64;
                                text_width + self.attr.padding.left + self.attr.padding.right
                            })
    }

    pub fn height(&self) -> f64 {
        self.height
            .unwrap_or_else(|| {
                                let text_height = self.layout.get_pixel_size().1 as f64;
                                text_height + self.attr.padding.top + self.attr.padding.bottom
                            })
    }

    pub fn set_width(&mut self, width: f64) {
        self.width = Some(width);
    }

    pub fn set_height(&mut self, height: f64) {
        self.height = Some(height);
    }

    pub fn stretch(&self) -> bool {
        self.stretch
    }

    pub fn render(&self, x: f64, y: f64) {
        self.context.save();
        self.context.translate(x, y + self.attr.padding.top);

        if let Some(ref bg_color) = self.attr.bg_color {
            bg_color.apply_to_context(&self.context);
            // FIXME: The use of `height` isnt' right here: we want to do the
            // full height of the bar, not the full height of the text. It
            // would be useful if we could do Surface.get_height(), but that
            // doesn't seem to be available in cairo-rs for some reason?
            self.context
                .rectangle(0.0, 0.0, self.width(), self.height());
            self.context.fill();
        }

        self.attr.fg_color.apply_to_context(&self.context);
        self.context
            .translate(self.attr.padding.left, self.attr.padding.top);
        self.context.show_pango_layout(&self.layout);

        self.context.restore();
    }
}
