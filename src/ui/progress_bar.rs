use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{vec4, Vec2, Vec4};

pub struct ProgressBar {
    /// The rectangle defining the button's position and size
    rect: Rectangle,
    /// How filled the progress bar is, [0, 1]
    progress: f32,
    background_color: Vec4,
    fill_color: Vec4,
    border: Option<(nalgebra_glm::Vec4, f32)>, // color, width
}

impl ProgressBar {
    pub fn new(size: Vec2) -> Self {
        Self {
            rect: Rectangle {
                pos: Vec2::zeros(),
                size,
            },
            progress: 0.0,
            background_color: vec4(1.0, 0.0, 1.0, 1.0),
            fill_color: vec4(1.0, 0.0, 1.0, 1.0),
            border: None,
        }
    }

    pub fn background_color(mut self, background_color: Vec4) -> Self {
        self.background_color = background_color;
        self
    }

    pub fn fill_color(mut self, fill_color: Vec4) -> Self {
        self.fill_color = fill_color;
        self
    }

    pub fn border(mut self, color: nalgebra_glm::Vec4, width: f32) -> Self {
        self.border = Some((color, width));
        self
    }

    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ProgressBar {
    fn update(&mut self, _app: &App, _msgq: &mut MsgQueue<Msg>) {}

    fn render(&self, app: &App) {
        // Draw background
        app.renderer.set_color(self.background_color);
        app.renderer.fill_rect(self.rect);

        // Draw filled
        let mut filled_rect = self.rect;
        filled_rect.size.x *= self.progress;
        app.renderer.set_color(self.fill_color);
        app.renderer.fill_rect(filled_rect);

        // Draw border
        if let Some((border_color, border_size)) = self.border {
            app.renderer.set_color(border_color);
            app.renderer.draw_rect(self.rect, border_size);
        }
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos
    }
}
