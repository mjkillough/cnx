use std::f64;
use std::rc::Rc;

use anyhow::{anyhow, Context, Result};
use ordered_float::OrderedFloat;
use xcb_util::ewmh;

use crate::text::{ComputedText, Text};
// use crate::widgets::{Widget, WidgetList};
// use crate::xcb::XcbEventStream;

fn get_root_visual_type(conn: &xcb::Connection, screen: &xcb::Screen<'_>) -> xcb::Visualtype {
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

/// Creates a `cairo::Surface` for the XCB window with the given `id`.
fn cairo_surface_for_xcb_window(
    conn: &xcb::Connection,
    screen: &xcb::Screen<'_>,
    id: u32,
    width: i32,
    height: i32,
) -> Result<cairo::XCBSurface> {
    let cairo_conn = unsafe {
        cairo::XCBConnection::from_raw_none(conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t)
    };
    let visual = unsafe {
        cairo::XCBVisualType::from_raw_none(
            &mut get_root_visual_type(conn, screen).base as *mut xcb::ffi::xcb_visualtype_t
                as *mut cairo_sys::xcb_visualtype_t,
        )
    };
    let drawable = cairo::XCBDrawable(id);
    let surface = cairo::XCBSurface::create(&cairo_conn, &drawable, &visual, width, height)
        .map_err(|status| anyhow!("XCBSurface::create: {}", status))?;
    Ok(surface)
}

fn create_surface(
    conn: &xcb::Connection,
    screen_idx: usize,
    window_id: u32,
    height: u16,
    width: Option<u16>,
    offset: Offset,
) -> Result<(u16, cairo::XCBSurface)> {
    let screen = conn
        .get_setup()
        .roots()
        .nth(screen_idx)
        .ok_or_else(|| anyhow!("Invalid screen"))?;
    let values = [
        (xcb::CW_BACK_PIXEL, screen.black_pixel()),
        (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE),
    ];

    let width = width.unwrap_or_else(|| screen.width_in_pixels());

    xcb::create_window(
        conn,
        xcb::COPY_FROM_PARENT as u8,
        window_id,
        screen.root(),
        offset.x,
        offset.y,
        width,
        height,
        0,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        screen.root_visual(),
        &values,
    );

    let surface = cairo_surface_for_xcb_window(
        conn,
        &screen,
        window_id,
        i32::from(width),
        i32::from(height),
    )?;

    Ok((width, surface))
}

/// An enum specifying the position of the Cnx bar.
///
/// Passed to [`Cnx::new()`] when constructing a [`Cnx`] instance.
///
/// [`Cnx::new()`]: struct.Cnx.html#method.new
/// [`Cnx`]: struct.Cnx.html
///
/// # Examples
///
/// ```
/// # use cnx::{Cnx, Position};
/// let mut cnx = Cnx::new(Position::Top);
/// ```
#[derive(Clone, Debug)]
pub enum Position {
    /// Position the Cnx bar at the top of the screen.
    Top,
    /// Position the Cnx bar at the bottom of the screen.
    Bottom,
}

/// A struct specifying the `x` and `y` offset
#[derive(Default, Clone, Copy)]
pub struct Offset {
    pub x: i16,
    pub y: i16,
}

pub struct Bar {
    position: Position,

    conn: Rc<ewmh::Connection>,
    screen_idx: usize,
    window_id: u32,

    surface: cairo::XCBSurface,
    width: u16,
    height: u16,
    offset: Offset,

    contents: Vec<Vec<ComputedText>>,
}

impl Bar {
    pub fn new(position: Position, width: Option<u16>, offset: Offset) -> Result<Bar> {
        let (conn, screen_idx) =
            xcb::Connection::connect(None).context("Failed to connect to X server")?;
        let screen_idx = screen_idx as usize;
        let window_id = conn.generate_id();

        // We don't actually care about how tall our initial window is - we'll resize
        // our window once we know how big it needs to be. However, it seems to need
        // to be bigger than 0px, or either Xcb/Cairo (or maybe QTile?) gets upset.
        let height = 1;
        let (width, surface) = create_surface(&conn, screen_idx, window_id, height, width, offset)?;

        let ewmh_conn = ewmh::Connection::connect(conn)
            .map_err(|(e, _)| e)
            .context("Failed to wrap xcb::Connection in ewmh::Connection")?;

        let bar = Bar {
            conn: Rc::new(ewmh_conn),
            window_id,
            screen_idx,
            surface,
            width,
            height,
            offset,
            position,
            contents: Vec::new(),
        };
        bar.set_ewmh_properties();

        // XXX We can't map the window until we've updated the window size, or nothing
        // gets rendered. I can't tell if this is something we're doing, something Cairo
        // is doing or something QTile is doing. This'll do for now and we'll see what
        // it is like with Lanta!
        // bar.map_window();
        bar.flush();

        Ok(bar)
    }

    fn flush(&self) {
        self.conn.flush();
    }

    fn map_window(&self) {
        xcb::map_window(&self.conn, self.window_id);
    }

    fn set_ewmh_properties(&self) {
        ewmh::set_wm_window_type(
            &self.conn,
            self.window_id,
            &[self.conn.WM_WINDOW_TYPE_DOCK()],
        );

        // TODO: Update _WM_STRUT_PARTIAL if the height/position of the bar changes?
        let mut strut_partial = ewmh::StrutPartial {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
            left_start_y: 0,
            left_end_y: 0,
            right_start_y: 0,
            right_end_y: 0,
            top_start_x: 0,
            top_end_x: 0,
            bottom_start_x: 0,
            bottom_end_x: 0,
        };
        match self.position {
            Position::Top => strut_partial.top = u32::from(self.height),
            Position::Bottom => strut_partial.bottom = u32::from(self.height),
        }
        ewmh::set_wm_strut_partial(&self.conn, self.window_id, strut_partial);
    }

    fn screen(&self) -> Result<xcb::Screen<'_>> {
        let screen = self
            .conn
            .get_setup()
            .roots()
            .nth(self.screen_idx)
            .ok_or_else(|| anyhow!("Invalid screen"))?;
        Ok(screen)
    }

    fn update_bar_height(&mut self, height: u16) -> Result<()> {
        if self.height != height {
            self.height = height;

            // If we're at the bottom of the screen, we'll need to update the
            // position of the window.
            let y = match self.position {
                Position::Top => self.offset.y.max(0) as u16,
                Position::Bottom => {
                    let h = (self.screen()?.height_in_pixels() - self.height) as i32;
                    h.checked_add(self.offset.y as i32).unwrap_or(h).max(0) as u16
                }
            };

            // Update the height/position of the XCB window and the height of the Cairo surface.
            let values = [
                (xcb::CONFIG_WINDOW_Y as u16, u32::from(y)),
                (xcb::CONFIG_WINDOW_HEIGHT as u16, u32::from(self.height)),
                (xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE),
            ];
            xcb::configure_window(&self.conn, self.window_id, &values);
            self.map_window();
            self.surface
                .set_size(i32::from(self.width), i32::from(self.height))
                .unwrap();

            // Update EWMH properties - we might need to reserve more or less space.
            self.set_ewmh_properties();
        }

        Ok(())
    }

    // Returns the connection to the X server.
    //
    // The owner of the `Bar` is responsible for polling this for events,
    // passing each to `Bar::process_event()`.
    pub fn connection(&self) -> &Rc<ewmh::Connection> {
        &self.conn
    }

    // Process an X event received from the `Bar::connection()`.
    pub fn process_event(&mut self, event: xcb::GenericEvent) -> Result<()> {
        let expose = event.response_type() & !0x80 == xcb::EXPOSE;
        if expose {
            println!("Redrawing entire bar - expose event.");
            self.redraw_entire_bar()?;
        }
        Ok(())
    }

    // Add a new widget's content to the `Bar`.
    //
    // Returns the index of the widget within the bar, so that subsequent
    // updates can be made by calling `Bar::update_content()`.
    pub fn add_content(&mut self, content: Vec<Text>) -> Result<usize> {
        let idx = self.contents.len();
        self.contents.push(Vec::new());
        self.update_content(idx, content)?;
        Ok(idx)
    }

    // Updates an existing widget's content in the `Bar`.
    pub fn update_content(&mut self, idx: usize, content: Vec<Text>) -> Result<()> {
        // If the text is the same, don't bother re-computing the text or
        // redrawing it. This is a spurious wake-up.
        let old = &self.contents[idx];
        if &content == old {
            return Ok(());
        }

        let mut new = content
            .into_iter()
            .map(|text| text.compute(&self.surface))
            .collect::<Result<Vec<_>>>()?;

        let error_margin = f64::EPSILON; // Use an epsilon for comparison

        // If there are any new texts or any non-stretch texts changed size,
        // we'll redraw all texts.
        let redraw_entire_bar = old.len() != new.len()
            || old
                .iter()
                .zip(&new)
                .any(|(old, new)| ((old.width - new.width).abs() < error_margin) && !new.stretch);

        // Steal dimenions from old ComputedText. If we need new dimensions,
        // they'll be recomputed by redraw_entire_bar().
        for (new, old) in new.iter_mut().zip(old.iter()) {
            new.x = old.x;
            new.y = old.y;
            new.height = old.height;
            // Only use width for stretch widgets.
            if new.stretch {
                new.width = old.width;
            }
        }

        self.contents[idx] = new;

        if !redraw_entire_bar {
            println!("Redrawing one");
            self.redraw_content(idx)?;
        } else {
            println!("Redrawing entire bar - widget update");
            self.redraw_entire_bar()?;
        }

        Ok(())
    }

    fn redraw_content(&mut self, idx: usize) -> Result<()> {
        for text in &mut self.contents[idx] {
            text.render(&self.surface)?;
        }

        self.flush();

        Ok(())
    }

    pub fn redraw_entire_bar(&mut self) -> Result<()> {
        self.recompute_dimensions()?;

        for idx in 0..self.contents.len() {
            self.redraw_content(idx)?;
        }
        Ok(())
    }

    fn recompute_dimensions(&mut self) -> Result<()> {
        // Set the height to the max height of any content.
        let height = self
            .contents
            .iter()
            .flatten()
            .map(|text| text.height)
            .max_by_key(|height| OrderedFloat(*height))
            .unwrap_or(0.0);
        for text in self.contents.iter_mut().flatten() {
            text.height = height;
        }
        self.update_bar_height(height as u16)?;

        // Sum the width of all non-stretch texts. Subtract from the screen
        // width to get width remaining for stretch texts.
        let used: f64 = self
            .contents
            .iter()
            .flatten()
            .filter(|text| !text.stretch)
            .map(|text| text.width)
            .sum();
        let remaining = f64::from(self.width) - used;

        // Distribute remaining width evenly between stretch texts.
        let stretches_count = self
            .contents
            .iter()
            .flatten()
            .filter(|text| text.stretch)
            .count();
        let stretch_width = remaining / (stretches_count as f64);
        let stretches = self
            .contents
            .iter_mut()
            .flatten()
            .filter(|text| text.stretch);
        for text in stretches {
            text.width = stretch_width;
        }

        // Set x based on computed widths.
        let mut x = 0.0;
        for text in self.contents.iter_mut().flatten() {
            text.x = x;
            x += text.width;
        }

        Ok(())
    }
}
