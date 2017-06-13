use text::{Attributes, Text};

use tokio_core::reactor::Handle;
use xcb_util::ewmh;


pub struct Pager {
    tokio_handle: Handle,
    active_attr: Attributes,
    inactive_attr: Attributes,
}

impl Pager {
    pub fn new(tokio_handle: Handle, active_attr: Attributes, inactive_attr: Attributes) -> Pager {
        Pager {
            tokio_handle,
            active_attr,
            inactive_attr,
        }
    }

    fn on_change(&self, conn: &ewmh::Connection, screen_idx: i32) -> Vec<Text> {
        let number = ewmh::get_number_of_desktops(conn, screen_idx)
            .get_reply()
            .unwrap() as usize;
        let current = ewmh::get_current_desktop(conn, screen_idx)
            .get_reply()
            .unwrap() as usize;
        let names_reply = ewmh::get_desktop_names(conn, screen_idx)
            .get_reply()
            .unwrap();
        let mut names = names_reply.strings();

        // EWMH states that `number` may not equal `names.len()`, as there may
        // be unnamed desktops, or more desktops than are currently in use.
        if names.len() > number {
            names.truncate(number);
        } else if number > names.len() {
            let num_unnamed = number - names.len();
            names.extend(vec!["?"; num_unnamed]);
        }

        names
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                let attr = if i == current {
                    self.active_attr.clone()
                } else {
                    self.inactive_attr.clone()
                };
                Text {
                    attr: attr,
                    text: name.to_owned(),
                    stretch: false,
                }
            })
            .collect()
    }
}

x_properties_widget!(Pager, tokio_handle, on_change; [
    NUMBER_OF_DESKTOPS,
    CURRENT_DESKTOP,
    DESKTOP_NAMES
]);
