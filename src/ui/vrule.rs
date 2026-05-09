use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{Vec2, Vec4};

use crate::ui::{msg::MsgQueue, widget::Widget};

pub struct VRule {
    pub color: Vec4,
    pub thickness: f32,
    rect: Rectangle,
}

impl VRule {
    pub fn new(color: Vec4, thickness: f32, height: f32) -> Self {
        Self {
            color,
            thickness,
            rect: Rectangle::new(0.0, 0.0, thickness, height),
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for VRule {
    fn update(&mut self, _app: &App, _msgq: &mut MsgQueue<Msg>) {}

    fn render(&self, app: &App) {
        app.renderer.set_color(self.color);
        app.renderer.fill_rect(self.rect);
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos;
    }
}
