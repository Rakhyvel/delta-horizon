use apricot::{app::App, rectangle::Rectangle, render_core::TextureId};
use nalgebra_glm::Vec2;

use crate::ui::{msg::MsgQueue, widget::Widget};

/// Represents a clickable button with a texture
pub struct TextureButton<Msg> {
    /// The rectangle defining the button's position and size
    rect: Rectangle,
    /// The texture ID for the normal state of the button
    texture_id: TextureId,
    /// The texture ID for the hovered state of the button
    hovered_texture_id: TextureId,
    /// Message to send when the button is clicked
    on_click: Option<Msg>,
}

impl<Msg> TextureButton<Msg> {
    /// Creates a new button
    pub fn new(rect: Rectangle, texture_id: TextureId, hovered_texture_id: TextureId) -> Self {
        Self {
            rect,
            texture_id,
            hovered_texture_id,
            on_click: None,
        }
    }

    pub fn on_click(mut self, on_click: Msg) -> Self {
        self.on_click = Some(on_click);
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for TextureButton<Msg> {
    /// Checks if the button is being hovered and clicked
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        let is_hovered = self.rect.contains_point(&app.mouse_pos);
        if is_hovered && app.mouse_left_clicked {
            if let Some(msg) = self.on_click.clone() {
                msgq.push(msg);
            }
        }
    }

    /// Renders a button to the screen
    fn render(&self, app: &App) {
        let is_hovered = self.rect.contains_point(&app.mouse_pos);
        app.renderer.copy_texture(
            self.rect,
            if is_hovered {
                self.hovered_texture_id
            } else {
                self.texture_id
            },
            Rectangle::new(0.0, 0.0, 360.0, 360.0),
        );
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos
    }
}
