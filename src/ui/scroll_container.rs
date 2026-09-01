use std::{
    cell::{Cell, RefCell},
    f32::consts::FRAC_PI_2,
    rc::Rc,
};

use apricot::{app::App, rectangle::Rectangle, render_core::MeshId};
use nalgebra_glm::{vec2, vec4, Vec2, Vec4};

use crate::{
    astro::epoch::EphemerisTime,
    scenes::events::Event,
    ui::{msg::MsgQueue, style::Style, widget::Widget},
};

pub struct ScrollContainer<Msg> {
    rect: Rectangle,
    child: Box<dyn Widget<Msg>>,
    offset: Vec2,
    content_size: Vec2,
}

impl<Msg> ScrollContainer<Msg> {
    pub fn new(size: Vec2, child: Box<dyn Widget<Msg>>) -> Self {
        Self {
            rect: Rectangle {
                pos: Vec2::zeros(),
                size,
            },
            child,
            offset: Vec2::zeros(),
            content_size: Vec2::zeros(),
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ScrollContainer<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        // Update child first, so that if it's another scroll container it gets the scroll first
        self.child.update(app, msgq);

        if app.mouse_wheel != 0.0
            && !app.is_wheel_consumed()
            && self.rect.contains_point(&app.mouse_pos)
        {
            app.consume_wheel();

            const SCROLL_SENSITIVITY: f32 = 50.0;
            self.offset.y -= SCROLL_SENSITIVITY * app.mouse_wheel;
            let max = (self.content_size.y - self.rect.size.y).max(0.0);
            self.offset.y = self.offset.y.clamp(0.0, max);
            self.layout(self.rect.pos);
        }
    }

    fn render(&self, app: &App) {
        app.renderer.set_scissor(Some(self.rect));
        self.child.render(app);
        app.renderer.set_scissor(None);
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos;
        self.child.layout(pos - self.offset);
        self.content_size = self.child.size()
    }
}
