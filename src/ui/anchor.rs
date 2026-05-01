use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{vec2, Vec2};

#[derive(Clone, Copy)]
pub enum AnchorPoint {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

pub struct Anchor<Msg> {
    rect: Rectangle,
    child: Box<dyn Widget<Msg>>,
    anchor: AnchorPoint,
}

impl<Msg: Clone + 'static> Anchor<Msg> {
    pub fn new(child: Box<dyn Widget<Msg>>, anchor: AnchorPoint) -> Self {
        let mut retval = Self {
            rect: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            child,
            anchor,
        };
        retval.layout(retval.rect.pos);
        retval
    }

    pub fn set_child(&mut self, child: Box<dyn Widget<Msg>>) {
        self.child = child;
        self.layout(self.rect.pos);
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Anchor<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        let w = app.window_size.x as f32;
        let h = app.window_size.y as f32;
        let size = self.child.size();

        let pos = match self.anchor {
            AnchorPoint::TopLeft => vec2(0.0, 0.0),
            AnchorPoint::TopCenter => vec2((w - size.x) / 2.0, 0.0),
            AnchorPoint::TopRight => vec2(w - size.x, 0.0),
            AnchorPoint::CenterLeft => vec2(0.0, (h - size.y) / 2.0),
            AnchorPoint::Center => vec2((w - size.x) / 2.0, (h - size.y) / 2.0),
            AnchorPoint::CenterRight => vec2(w - size.x, (h - size.y) / 2.0),
            AnchorPoint::BottomLeft => vec2(0.0, h - size.y),
            AnchorPoint::BottomCenter => vec2((w - size.x) / 2.0, h - size.y),
            AnchorPoint::BottomRight => vec2(w - size.x, h - size.y),
        };

        self.rect.pos = pos;
        self.child.layout(pos);
        self.child.as_mut().update(app, msgq);
    }

    fn render(&self, app: &App) {
        self.child.render(app);
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.child.layout(pos);

        self.rect.size = self.child.size();
    }
}
