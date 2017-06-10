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

        let cr = Context::new(&self.surface);
        cr.set_source_rgb(1.0, 0.0, 0.0);

        let layout = cr.create_pango_layout();
        layout.set_text("Hello, world!", 13);
        layout.set_font_description(Some(&font));

        cr.show_pango_layout(&layout);

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
