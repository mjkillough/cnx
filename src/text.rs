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
}


#[derive(Clone)]
pub struct Attributes {
    pub font: FontDescription,
    pub fg_color: Color,
    pub bg_color: Option<Color>,
    pub padding: Padding,
}

impl Attributes {
    #[allow(dead_code)]
    pub fn with_font(&self, font: FontDescription) -> Attributes {
        let mut new = self.clone();
        new.font = font;
        new
    }

    #[allow(dead_code)]
    pub fn with_fg_color(&self, fg_color: Color) -> Attributes {
        let mut new = self.clone();
        new.fg_color = fg_color;
        new
    }

    #[allow(dead_code)]
    pub fn with_bg_color(&self, bg_color: Option<Color>) -> Attributes {
        let mut new = self.clone();
        new.bg_color = bg_color;
        new
    }

    #[allow(dead_code)]
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
    fn create_contexts(&self, surface: &Surface) -> (Context, Layout) {
        let context = Context::new(&surface);

        let layout = context.create_pango_layout();
        layout.set_text(&self.text, self.text.len() as i32);
        layout.set_font_description(Some(&self.attr.font));

        (context, layout)
    }

    fn width_and_height_for_layout(&self, layout: &Layout) -> (f64, f64) {
        let padding = &self.attr.padding;
        let (text_width, text_height) = layout.get_pixel_size();
        let width = text_width as f64 + padding.left + padding.right;
        let height = text_height as f64 + padding.top + padding.bottom;
        (width, height)
    }

    pub fn compute_width_and_height(&self, surface: &Surface) -> (f64, f64) {
        let (_, layout) = self.create_contexts(surface);
        self.width_and_height_for_layout(&layout)
    }

    pub fn render(
        &self,
        surface: &Surface,
        x: f64,
        y: f64,
        width: Option<f64>,
        height: Option<f64>,
    ) -> (f64, f64) {
        let (context, layout) = self.create_contexts(surface);
        context.translate(x, y);

        // The `width`/`height` parameters allow the caller to override how big we'll draw
        // this block of text. If they are not specified, then we'll default to whatever
        // width/height the text actually takes.
        let (layout_width, layout_height) = self.width_and_height_for_layout(&layout);
        let width = width.unwrap_or(layout_width);
        let height = height.unwrap_or(layout_height);

        if let Some(ref bg_color) = self.attr.bg_color {
            bg_color.apply_to_context(&context);
            // FIXME: The use of `height` isnt' right here: we want to do the
            // full height of the bar, not the full height of the text. It
            // would be useful if we could do Surface.get_height(), but that
            // doesn't seem to be available in cairo-rs for some reason?
            context.rectangle(0.0, 0.0, width, height);
            context.fill();
        }

        self.attr.fg_color.apply_to_context(&context);
        context.translate(self.attr.padding.left, self.attr.padding.top);
        context.show_pango_layout(&layout);

        (width, height)
    }
}
