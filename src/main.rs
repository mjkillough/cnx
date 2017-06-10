extern crate chrono;
extern crate xcb;
extern crate xcb_util;
extern crate cairo;
extern crate cairo_sys;
extern crate pango;
extern crate pangocairo;

use std::rc::Rc;

use cairo::{Context, Surface, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use pango::LayoutExt;
use pangocairo::CairoContextExt;
use xcb::ffi::*;
use chrono::prelude::*;


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
    fn white() -> Color {
        Color {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
        }
    }

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

impl TextAttributes {
    fn with_font(&self, font: pango::FontDescription) -> TextAttributes {
        let mut new = self.clone();
        new.font = font;
        new
    }

    fn with_fg_color(&self, fg_color: Color) -> TextAttributes {
        let mut new = self.clone();
        new.fg_color = fg_color;
        new
    }

    fn with_bg_color(&self, bg_color: Option<Color>) -> TextAttributes {
        let mut new = self.clone();
        new.bg_color = bg_color;
        new
    }

    fn with_padding(&self, padding: Padding) -> TextAttributes {
        let mut new = self.clone();
        new.padding = padding;
        new
    }
}


struct Text {
    attr: TextAttributes,
    text: String,
    stretch: bool,
}

impl Text {
    fn layout(self, surface: &Surface) -> TextLayout {
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


struct TextLayout {
    attr: TextAttributes,
    stretch: bool,
    width: Option<f64>,
    height: Option<f64>,
    context: Context,
    layout: pango::Layout,
}

impl TextLayout {
    fn width(&self) -> f64 {
        self.width
            .unwrap_or_else(|| {
                                let text_width = self.layout.get_pixel_size().0 as f64;
                                text_width + self.attr.padding.left + self.attr.padding.right
                            })
    }

    fn height(&self) -> f64 {
        self.height
            .unwrap_or_else(|| {
                                let text_height = self.layout.get_pixel_size().1 as f64;
                                text_height + self.attr.padding.top + self.attr.padding.bottom
                            })
    }

    fn set_width(&mut self, width: f64) {
        self.width = Some(width);
    }

    fn set_height(&mut self, height: f64) {
        self.height = Some(height);
    }

    fn stretch(&self) -> bool {
        self.stretch
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


struct Pager {
    conn: xcb_util::ewmh::Connection,
    screen_idx: i32,

    active_attr: TextAttributes,
    inactive_attr: TextAttributes,
}

impl Pager {
    fn new(active_attr: TextAttributes, inactive_attr: TextAttributes) -> Pager {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
        let ewmh_conn = xcb_util::ewmh::Connection::connect(conn)
            .map_err(|_| ())
            .unwrap();

        Pager {
            conn: ewmh_conn,
            screen_idx: screen_idx,
            active_attr: active_attr,
            inactive_attr: inactive_attr,
        }
    }

    fn get_desktops_info(&self) -> Vec<(bool, String)> {
        let number = xcb_util::ewmh::get_number_of_desktops(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap() as usize;
        let current = xcb_util::ewmh::get_current_desktop(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap() as usize;
        let names_reply = xcb_util::ewmh::get_desktop_names(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap();
        let mut names = names_reply.strings();

        // EWMH states that `number` may not equal `names.len()`, as there may
        // be unnamed desktops, or more desktops than are currently in use.
        if names.len() > number {
            names.truncate(number);
        } else if number > names.len() {
            for i in 0..(number - names.len()) {
                names.push("?");
            }
        }

        names
            .into_iter()
            .enumerate()
            .map(|(i, name)| (i == current, name.to_owned()))
            .collect()
    }

    fn compute_text(&self) -> Vec<Text> {
        let desktops = self.get_desktops_info();
        self.get_desktops_info()
            .into_iter()
            .map(|(active, name)| {
                Text {
                    attr: if active {
                        self.active_attr.clone()
                    } else {
                        self.inactive_attr.clone()
                    },
                    text: name,
                    stretch: false,
                }
            })
            .collect()
    }
}


struct ActiveWindowTitle {
    conn: xcb_util::ewmh::Connection,
    screen_idx: i32,

    attr: TextAttributes,
}

impl ActiveWindowTitle {
    fn new(attr: TextAttributes) -> ActiveWindowTitle {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();
        let ewmh_conn = xcb_util::ewmh::Connection::connect(conn)
            .map_err(|_| ())
            .unwrap();

        ActiveWindowTitle {
            conn: ewmh_conn,
            screen_idx: screen_idx,

            attr: attr,
        }
    }

    fn get_active_window_title(&self) -> String {
        let active_window = xcb_util::ewmh::get_active_window(&self.conn, self.screen_idx)
            .get_reply()
            .unwrap();
        let reply = xcb_util::ewmh::get_wm_name(&self.conn, active_window).get_reply();

        match reply {
            Ok(inner) => inner.string().to_owned(),
            // Probably means there's no window focused, or it doesn't have _NET_WM_NAME:
            Err(_) => "".to_owned(),
        }
    }

    fn compute_text(&self) -> Vec<Text> {
        vec![Text {
                 attr: self.attr.clone(),
                 text: self.get_active_window_title(),
                 stretch: true,
             }]
    }
}


struct Clock {
    conn: xcb::Connection,

    attr: TextAttributes,
}

impl Clock {
    fn new(attr: TextAttributes) -> Clock {
        let (conn, screen_idx) = xcb::Connection::connect_with_xlib_display().unwrap();

        Clock {
            conn: conn,
            attr: attr,
        }
    }

    fn compute_text(&self) -> Vec<Text> {
        let current_time = Local::now().format("%Y-%m-%d %a %I:%M %p").to_string();
        vec![Text {
                 attr: self.attr.clone(),
                 text: current_time,
                 stretch: false,
             }]
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
        let inactive_attr = TextAttributes {
            font: pango::FontDescription::from_string("Envy Code R 27"),
            fg_color: Color::white(),
            bg_color: None,
            padding: Padding::new(10.0, 10.0, 5.0, 5.0),
        };
        let active_attr = inactive_attr.with_bg_color(Some(Color::blue()));

        let mut texts = Pager::new(active_attr, inactive_attr.clone()).compute_text();
        texts.extend(ActiveWindowTitle::new(inactive_attr.clone()).compute_text());
        texts.extend(Clock::new(inactive_attr.clone()).compute_text());

        self.render_text_blocks(texts);

        self.conn.flush();
    }

    fn render_text_blocks(&self, texts: Vec<Text>) {
        // Layout each block of text. After this, we can query the width of each
        // block, which will allow us to do more complex layout below.
        let mut layouts: Vec<_> = texts.into_iter().map(|t| t.layout(&self.surface)).collect();

        // Calculate how much free space we have after laying out all the
        // non-stretch blocks. Split the remaining space (if any) between the
        // stretch blocks. If there isn't enough space for the non-stretch blocks
        // do nothing and allow it to overflow.
        {
            let mut width = 0.0;
            let mut stretched = Vec::new();
            for layout in layouts.iter_mut() {
                if !layout.stretch() {
                    width += layout.width();
                } else {
                    stretched.push(layout);
                }
            }

            let remaining_width = self.screen().width_in_pixels() as f64 - width;
            let remaining_width = if remaining_width < 0.0 {
                0.0
            } else {
                remaining_width
            };
            let width_per_stretched = remaining_width / (stretched.len() as f64);
            for layout in stretched.iter_mut() {
                layout.set_width(width_per_stretched);
            }
        }

        // TODO: Set the height of the window and the height of each text block(?)
        // to the height of the largest bit of text.

        // Finally, just render each block of text in turn.
        let mut x = 0.0;
        for layout in &layouts {
            layout.render(x, 0.0);
            x += layout.width();
        }
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
