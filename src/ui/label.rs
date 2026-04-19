use std::marker::PhantomData;

use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::{
    app::App,
    font::{Font, FontId},
    rectangle::Rectangle,
};
use nalgebra_glm::Vec2;

/// A button with text
pub struct Label<Msg> {
    /// The rectangle defining the button's position and size
    rect: Rectangle,
    /// The text to be drawn for the button
    label: String,
    phantom: PhantomData<Msg>,
}

impl<Msg> Label<Msg> {
    /// Creates a label
    pub fn new(label: impl Into<String>, font: &Font) -> Self {
        let text = label.into();
        let size = font.measure(&text);
        let rect = Rectangle {
            pos: Vec2::zeros(),
            size,
        };
        Self {
            rect,
            label: text,
            phantom: PhantomData::<Msg> {},
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Label<Msg> {
    fn update(&mut self, _app: &App, _msgq: &mut MsgQueue<Msg>) {}

    fn render(&self, app: &App) {
        app.renderer.draw_text(self.rect.pos, &self.label);
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos
    }
}
