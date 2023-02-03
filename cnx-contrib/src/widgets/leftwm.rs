use anyhow::Result;
use cnx::text::{Attributes, Text};
use cnx::widgets::{Widget, WidgetStream};
use process_stream::{Process, ProcessExt, StreamExt};
use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
struct State {
    // _window_title: String,
    workspaces: Vec<Workspace>,
}

#[derive(Deserialize, Debug)]
struct Workspace {
    // h: u16,
    // w: u16,
    // x: i16,
    // y: i16,
    output: String,
    // layout: String,
    // index: u16,
    tags: Vec<Tag>,
}

#[derive(Deserialize, Debug)]
struct Tag {
    name: String,
    // index: u16,
    mine: bool,
    visible: bool,
    focused: bool,
    // TODO: maybe use?
    // urgent: bool,
    busy: bool,
}

/// LeftWMAttributes represents the different [`Attributes`] used by the different tag states
#[derive(Clone)]
pub struct LeftWMAttributes {
    /// The Attributes of the focused tag
    pub focused: Attributes,
    /// The Attributes of the visible tags
    pub visible: Attributes,
    /// The Attributes of the busy tags
    pub busy: Attributes,
    /// The Attributes of the empty tags
    pub empty: Attributes,
}

/// LeftWM widget that shows information about the worksapces and tags
pub struct LeftWM {
    output: String,
    attrs: LeftWMAttributes,
}

impl LeftWM {
    /// Creates a new [`LeftWM`] widget.
    ///
    /// Arguments
    ///
    /// * `output` - Represents the name of the monitor this widget it attached to
    ///
    /// * `attr` - Represents the [`LeftWMAttributes`] which controls properties like
    /// `Font`, foreground and background color of the different tag's states
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate cnx;
    /// #
    /// # use cnx::*;
    /// # use cnx::text::*;
    /// # use cnx_contrib::widgets::leftwm::*;
    /// # use anyhow::Result;
    /// #
    /// # fn run() -> Result<()> {
    /// let focused = Attributes {
    ///     font: Font::new("SourceCodePro 14"),
    ///     fg_color: Color::white(),
    ///     bg_color: Some(Color::blue()),
    ///     padding: Padding::new(8.0, 8.0, 0.0, 0.0),
    /// };
    ///
    /// let empty = Attributes {
    ///     bg_color: None,
    ///     ..focused.clone()
    /// };
    /// let busy = Attributes {
    ///     fg_color: Color::blue(),
    ///     ..empty.clone()
    /// };
    /// let visible = Attributes {
    ///     fg_color: Color::red(),
    ///     ..empty.clone()
    /// };
    ///
    /// let mut cnx = Cnx::new(Position::Top);
    /// let leftwm_attr = LeftWMAttributes {
    ///     focused,
    ///     empty,
    ///     busy,
    ///     visible,
    /// };
    ///
    /// cnx.add_widget(LeftWM::new("eDP1".to_string(), leftwm_attr));
    /// # Ok(())
    /// # }
    /// # fn main() { run().unwrap(); }
    /// ```
    pub fn new(output: String, attrs: LeftWMAttributes) -> Self {
        LeftWM { output, attrs }
    }

    fn on_change(&self, content: String) -> Result<Vec<Text>> {
        let state: State = serde_json::from_str(&content)?;
        let w = state.workspaces.iter().find(|w| w.output == self.output);
        if let Some(w) = w {
            let text = w
                .tags
                .iter()
                .map(|t| {
                    let attr = if t.mine && t.focused {
                        self.attrs.focused.clone()
                    } else if t.mine && t.visible {
                        self.attrs.visible.clone()
                    } else if t.busy {
                        self.attrs.busy.clone()
                    } else {
                        self.attrs.empty.clone()
                    };
                    Text {
                        attr,
                        text: t.name.clone(),
                        stretch: false,
                        markup: true,
                    }
                })
                .collect();
            Ok(text)
        } else {
            Ok(vec![])
        }
    }
}

impl Widget for LeftWM {
    fn into_stream(self: Box<Self>) -> Result<WidgetStream> {
        let mut state = Process::new("leftwm-state");
        let s = state
            .spawn_and_stream()?
            .map(move |s| self.on_change(s.to_string()));
        // let s = s.map(|s| todo!());
        Ok(Box::pin(s))
    }
}
