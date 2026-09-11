use apricot::{app::App, rectangle::Rectangle, render_core::MeshId};
use nalgebra_glm::{vec2, Vec2, Vec4};

use crate::ui::{msg::MsgQueue, widget::Widget};

pub struct Shape {
    rect: Rectangle,
    mesh: MeshId,
    rotation: f32,
    color: Vec4,
    radius: f32,
}

impl Shape {
    pub fn new(size: Vec2, mesh: MeshId, rotation: f32, color: Vec4, radius: f32) -> Self {
        Self {
            rect: Rectangle {
                pos: vec2(0.0, 0.0),
                size,
            },
            mesh,
            rotation,
            color,
            radius,
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Shape {
    fn update(&mut self, _app: &App, _msgq: &mut MsgQueue<Msg>) {}

    fn render(&self, app: &App) {
        app.renderer.set_color(self.color);
        app.renderer.fill_polygon(
            self.mesh,
            self.rect.pos + self.rect.size * 0.5,
            self.radius,
            self.rotation,
        );
    }

    fn size(&self) -> nalgebra_glm::Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: nalgebra_glm::Vec2) {
        self.rect.pos = pos;
    }
}
