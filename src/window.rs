use std::rc::Rc;

use xcb;
use cairo::{self, XCBSurface};
use cairo_sys;

use text::Text;


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
pub struct Window {
    conn: Rc<xcb::Connection>,
    screen_idx: usize,
    surface: cairo::Surface,
}

impl Window {
    pub fn new(conn: Rc<xcb::Connection>, screen_idx: usize) -> Window {
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

            unsafe {
                let cairo_conn = cairo::XCBConnection::from_raw_none(conn.get_raw_conn() as
                                                                     *mut cairo_sys::xcb_connection_t);
                let visual =
                    cairo::XCBVisualType::from_raw_none(&mut get_root_visual_type(&conn, &screen).base as
                                                        *mut xcb::ffi::xcb_visualtype_t as
                                                        *mut cairo_sys::xcb_visualtype_t);
                let drawable = cairo::XCBDrawable(id);
                cairo::Surface::create(&cairo_conn, &drawable, &visual, width as i32, height as i32)
                // TODO: Update surface width/height when window size changes.
            }
        };

        xcb::map_window(&conn, id);
        conn.flush();

        Window {
            conn,
            screen_idx,
            surface,
        }
    }

    fn screen(&self) -> xcb::Screen {
        self.conn
            .get_setup()
            .roots()
            .nth(self.screen_idx)
            .expect("Invalid screen")
    }

    pub fn expose(&self, texts: Vec<Text>) {
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
            for layout in &mut layouts {
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
            for layout in &mut stretched {
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
