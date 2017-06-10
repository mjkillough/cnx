extern crate xcb;
extern crate cairo;
extern crate cairo_sys;
extern crate pango;
extern crate pangocairo;

use std::rc::Rc;

use cairo::{Context, Surface, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use pango::LayoutExt;
use pangocairo::CairoContextExt;
use xcb::ffi::*;


fn get_root_visual_type(conn: &xcb::Connection, screen: &xcb::Screen) -> xcb::Visualtype {
    for root in conn.get_setup().roots() {
        for allowed_depth in root.allowed_depths() {
            for visual in allowed_depth.visuals() {
                if visual.visual_id() == screen.root_visual() {
                    return visual;
                }
            }
        }
    }
    panic!("No visual type found");
}


#[derive(Clone, Debug)]
struct Color {
    red: f64,
    green: f64,
    blue: f64,
}

impl Color {
    fn red() -> Color {
        Color {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
        }
    }

    fn blue() -> Color {
        Color {
            red: 0.0,
            green: 0.0,
            blue: 1.0,
        }
    }

    fn apply_to_context(&self, cr: &Context) {
        cr.set_source_rgb(self.red, self.green, self.blue);
    }
}

#[derive(Clone, Debug)]
struct Padding {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Padding {
    fn none() -> Padding {
        Padding {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }
    fn new(left: f64, right: f64, top: f64, bottom: f64) -> Padding {
        Padding {
            left,
            right,
            top,
            bottom,
        }
    }

    fn uniform(value: f64) -> Padding {
        Padding {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}


#[derive(Clone)]
struct TextAttributes {
    font: pango::FontDescription,
    fg_color: Color,
    bg_color: Option<Color>,
    padding: Padding,
}


struct Text {
    attr: TextAttributes,
    text: String,
}

impl Text {
    fn layout(self, surface: &Surface) -> TextLayout {
        let context = Context::new(&surface);

        let layout = context.create_pango_layout();
        layout.set_text(&self.text, self.text.len() as i32);
        layout.set_font_description(Some(&self.attr.font));

        TextLayout {
            attr: self.attr.clone(),
            context: context,
            layout: layout,
        }
    }
}


struct TextLayout {
    attr: TextAttributes,
    context: Context,
    layout: pango::Layout,
}

impl TextLayout {
    fn width(&self) -> f64 {
        let text_width = self.layout.get_pixel_size().0 as f64;
        text_width + self.attr.padding.left + self.attr.padding.right
    }

    fn height(&self) -> f64 {
        let text_height = self.layout.get_pixel_size().1 as f64;
        text_height + self.attr.padding.top + self.attr.padding.bottom
    }

    fn render(&self, x: f64, y: f64) {
        self.context.save();
        self.context.translate(x, y + self.attr.padding.top);

        if let Some(ref bg_color) = self.attr.bg_color {
            bg_color.apply_to_context(&self.context);
            // FIXME: The use of `height` isnt' right here: we want to do the
            // full height of the bar, not the full height of the text. It
            // would be useful if we could do Surface.get_height(), but that
            // doesn't seem to be available in cairo-rs for some reason?
            self.context
                .rectangle(0.0,
                           0.0,
                           self.width() + self.attr.padding.right,
                           self.height() + self.attr.padding.bottom);
            self.context.fill();
        }

        self.attr.fg_color.apply_to_context(&self.context);
        self.context
            .translate(self.attr.padding.left, self.attr.padding.right);
        self.context.show_pango_layout(&self.layout);

        self.context.restore();
    }
}


// TODO: impl Drop?
struct Window {
    conn: Rc<xcb::Connection>,
    screen_idx: usize,
    id: u32,
    surface: Surface,
}

impl Window {
    fn new(conn: Rc<xcb::Connection>, screen_idx: usize) -> Window {
        let id = conn.generate_id();

        let surface = {
            let screen = conn.get_setup()
                .roots()
                .nth(screen_idx)
                .expect("invalid screen");
            let values = [(xcb::CW_BACK_PIXEL, screen.black_pixel()),
                          (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE)];

            let (width, height) = (screen.width_in_pixels(), 100);

            xcb::create_window(&conn,
                               xcb::COPY_FROM_PARENT as u8,
                               id,
                               screen.root(),
                               0,
                               0,
                               width,
                               height,
                               10,
                               xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
                               screen.root_visual(),
                               &values);

            let surface = unsafe {
                let cairo_conn = XCBConnection::from_raw_none(conn.get_raw_conn() as
                                                              *mut cairo_sys::xcb_connection_t);
                let visual =
                    XCBVisualType::from_raw_none(&mut get_root_visual_type(&conn, &screen).base as
                                                 *mut xcb::ffi::xcb_visualtype_t as
                                                 *mut cairo_sys::xcb_visualtype_t);
                let drawable = XCBDrawable(id);
                Surface::create(&cairo_conn, &drawable, &visual, width as i32, height as i32)
                // TODO: Update surface width/height when window size changes.
            };

            surface
        };

        xcb::map_window(&conn, id);
        conn.flush();

        Window {
            conn,
            screen_idx,
            id,
            surface,
        }
    }

    fn screen<'a>(&'a self) -> xcb::Screen<'a> {
        self.conn
            .get_setup()
            .roots()
            .nth(self.screen_idx)
            .expect("Invalid screen")
    }

    fn expose(&self) {
        let font = pango::FontDescription::from_string("Envy Code R 27");

        let attr1 = TextAttributes {
            font: font.clone(),
            fg_color: Color::red(),
            bg_color: None,
            padding: Padding::uniform(5.0),
        };
        let attr2 = TextAttributes {
            font: font.clone(),
            fg_color: Color::blue(),
            bg_color: Some(Color::red()),
            padding: Padding::uniform(5.0),
        };
        let texts = vec![Text {
                             attr: attr1,
                             text: "Hello, world!".to_owned(),
                         },
                         Text {
                             attr: attr2,
                             text: "Again".to_owned(),
                         }];
        let layouts: Vec<_> = texts.into_iter().map(|t| t.layout(&self.surface)).collect();
        let mut x = 0.0;
        for layout in layouts {
            layout.render(x, 0.0);
            x += layout.width();
        }

        self.conn.flush();
    }
}


fn main() {
    let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
    let conn = Rc::new(conn);

    let w = Window::new(conn.clone(), screen_idx as usize);

    loop {
        let event = conn.wait_for_event();
        match event {
            None => {
                break;
            }
            Some(event) => {
                let r = event.response_type() & !0x80;
                match r {
                    xcb::EXPOSE => w.expose(),
                    _ => {}
                }
            }
        }
    }
}
