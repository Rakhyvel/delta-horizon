use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::{app::App, font::FontId, rectangle::Rectangle};
use nalgebra_glm::Vec2;

/// A button with text
pub struct Label {
    /// The rectangle defining the button's position and size
    rect: Rectangle,
    /// The text to be drawn for the button
    label: String,
    font_id: Option<FontId>,
}

impl Label {
    /// Creates a label
    pub fn new(label: impl Into<String>) -> Self {
        let text = label.into();
        let rect = Rectangle {
            pos: Vec2::zeros(),
            size: Vec2::zeros(),
        };
        Self {
            rect,
            label: text,
            font_id: None,
        }
    }

    pub fn font(mut self, font_id: FontId, app: &App) -> Self {
        self.font_id = Some(font_id);
        let font = app.renderer.get_font_from_id(font_id).unwrap();
        let size = font.measure(&self.label);
        self.rect.size = size;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Label {
    fn update(&mut self, _app: &App, _msgq: &mut MsgQueue<Msg>) {}

    fn render(&self, app: &App) {
        let old_font = app.renderer.get_current_font_id();

        app.renderer.set_font(self.font_id.unwrap());
        app.renderer.draw_text(self.rect.pos, &self.label);

        if let Some(old_font) = old_font {
            app.renderer.set_font(old_font);
        }
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos
    }
}
