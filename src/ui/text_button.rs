use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::Vec4;

/// A button with text
pub struct TextButton<Msg> {
    /// The rectangle defining the button's position and size
    rect: Rectangle,
    /// The text to be drawn for the button
    label: String,
    /// The background color of the button
    color: Vec4,
    /// The background color of the button when hovered
    hovered_color: Vec4,
    /// The message to send when the button is clicked (no message if None)
    on_click: Option<Msg>,
}

impl<Msg> TextButton<Msg> {
    /// Creates a textu button
    pub fn new(
        rect: Rectangle,
        label: impl Into<String>,
        color: Vec4,
        hovered_color: Vec4,
    ) -> Self {
        Self {
            rect,
            label: label.into(),
            color,
            hovered_color,
            on_click: None,
        }
    }

    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for TextButton<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        if self.rect.contains_point(&app.mouse_pos) && app.mouse_left_clicked {
            if let Some(msg) = &self.on_click {
                msgq.push(msg.clone());
            }
        }
    }

    fn render(&self, app: &App) {
        let color = if self.rect.contains_point(&app.mouse_pos) {
            self.hovered_color
        } else {
            self.color
        };
        app.renderer.set_color(color);
        app.renderer.fill_rect(self.rect);
        app.renderer.draw_text(self.rect.pos, &self.label);
    }
}
