use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{vec2, Vec2};

pub struct Modal<Msg> {
    rect: Rectangle,
    child: Box<dyn Widget<Msg>>,
    shown: bool,
}

impl<Msg: Clone + 'static> Modal<Msg> {
    pub fn new(child: Box<dyn Widget<Msg>>) -> Self {
        let mut retval = Self {
            rect: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            child,
            shown: false,
        };
        retval.layout(retval.rect.pos);
        retval
    }

    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self.layout(self.rect.pos);
        self
    }

    pub fn set_shown(&mut self, shown: bool) {
        self.shown = shown
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Modal<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        let w = app.window_size.x as f32;
        let h = app.window_size.y as f32;
        let size = self.child.size();

        let pos = vec2((w - size.x) / 2.0, (h - size.y) / 2.0);

        self.rect.pos = pos;
        self.child.layout(pos);
        self.child.as_mut().update(app, msgq);
    }

    fn render(&self, app: &App) {
        if self.shown {
            self.child.render(app);
        }
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.child.layout(pos);

        self.rect.size = self.child.size();
    }
}
