use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::Vec2;

use crate::ui::{msg::MsgQueue, widget::Widget};

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
        app.push_clip(self.rect);
        self.child.update(app, msgq);
        app.pop_clip();

        if app.mouse_wheel != 0.0 && !app.is_wheel_consumed() && app.mouse_over(&self.rect) {
            app.consume_wheel();

            const SCROLL_SENSITIVITY: f32 = 50.0;
            self.offset.y -= SCROLL_SENSITIVITY * app.mouse_wheel;
            let max = (self.content_size.y - self.rect.size.y).max(0.0);
            self.offset.y = self.offset.y.clamp(0.0, max);
            self.layout(self.rect.pos);
        }
    }

    fn render(&self, app: &App) {
        app.push_clip(self.rect);
        app.renderer.set_scissor(Some(self.rect));
        self.child.render(app);
        app.pop_clip();
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
