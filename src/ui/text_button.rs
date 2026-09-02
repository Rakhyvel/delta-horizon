use crate::ui::{msg::MsgQueue, style::Style, widget::Widget};
use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{vec4, Vec2, Vec4};

/// A button with text
pub struct TextButton<Msg> {
    /// The rectangle defining the button's position and size
    rect: Rectangle,
    /// The text to be drawn for the button
    label: String,
    text_color: Vec4,
    inactive_text_color: Vec4,
    /// The background color of the button
    background_color: Vec4,
    inactive_background_color: Vec4,
    /// The background color of the button when hovered
    hovered_color: Vec4,
    border: Option<(nalgebra_glm::Vec4, f32)>,
    inactive_border: Option<(nalgebra_glm::Vec4, f32)>,
    /// The message to send when the button is clicked (no message if None)
    on_click: Option<Msg>,
    /// Whether or not the button is active or nah
    active: bool,
    /// Whether the button is hovered or not
    hovered: bool,
}

impl<Msg> TextButton<Msg> {
    /// Creates a textu button
    pub fn new(rect: Rectangle, label: impl Into<String>) -> Self {
        Self {
            rect,
            label: label.into(),
            text_color: vec4(1.0, 1.0, 1.0, 1.0),
            inactive_text_color: vec4(1.0, 1.0, 1.0, 1.0),
            background_color: vec4(1.0, 0.0, 1.0, 255.0),
            inactive_background_color: vec4(1.0, 0.0, 1.0, 255.0),
            hovered_color: vec4(1.0, 0.0, 1.0, 255.0),
            border: None,
            inactive_border: None,
            on_click: None,
            active: true,
            hovered: false,
        }
    }

    #[allow(dead_code)]
    pub fn background_color(mut self, background_color: Vec4) -> Self {
        self.background_color = background_color;
        self
    }

    #[allow(dead_code)]
    pub fn hovered_color(mut self, hovered_color: Vec4) -> Self {
        self.hovered_color = hovered_color;
        self
    }

    pub fn use_style(mut self, style: &Style) -> Self {
        self.text_color = style.text_primary;
        self.inactive_text_color = style.text_disabled;
        self.background_color = style.btn_active_bg;
        self.inactive_background_color = style.btn_inactive_bg;
        self.border = Some((style.btn_active_border, 1.0));
        self.inactive_border = Some((style.btn_inactive_border, 1.0));
        self.hovered_color = style.btn_hover_bg;
        self
    }

    pub fn use_style_accented(mut self, style: &Style) -> Self {
        self.text_color = style.text_primary;
        self.inactive_text_color = style.text_disabled;
        self.background_color = style.btn_accent_bg;
        self.inactive_background_color = style.btn_inactive_bg;
        self.border = Some((style.btn_accent_border, 1.0));
        self.inactive_border = Some((style.btn_inactive_border, 1.0));
        self.hovered_color = style.btn_accent_hover_bg;
        self
    }

    #[allow(dead_code)]
    pub fn use_style_if(self, style: &Style, condition: bool) -> Self {
        if !condition {
            return self;
        }
        self.use_style(style)
    }

    #[allow(dead_code)]
    pub fn use_style_accented_if(self, style: &Style, condition: bool) -> Self {
        if !condition {
            return self;
        }
        self.use_style_accented(style)
    }

    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for TextButton<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        self.hovered = self.active && self.rect.contains_point(&app.mouse_pos);

        if self.active && self.hovered && !app.is_click_consumed() && app.mouse_left_clicked {
            if let Some(msg) = &self.on_click {
                msgq.push(msg.clone());
                app.consume_click();
            }
        }
    }

    fn render(&self, app: &App) {
        // Measure text size
        let text_size = {
            let current_font = app.renderer.get_current_font().unwrap();
            current_font.measure(&self.label)
        };

        // Draw background
        let color = if !self.active {
            self.inactive_background_color
        } else if self.hovered {
            self.hovered_color
        } else {
            self.background_color
        };
        app.renderer.set_color(color);
        app.renderer.fill_rect(self.rect);

        // Draw border
        if self.active {
            if let Some((border_color, border_size)) = self.border {
                app.renderer.set_color(border_color);
                app.renderer.draw_rect(self.rect, border_size);
            }
        } else {
            if let Some((border_color, border_size)) = self.inactive_border {
                app.renderer.set_color(border_color);
                app.renderer.draw_rect(self.rect, border_size);
            }
        }

        // Draw text
        if self.active {
            app.renderer.set_color(self.text_color);
        } else {
            app.renderer.set_color(self.inactive_text_color);
        }
        app.renderer.draw_text(
            self.rect.pos + (self.rect.size - text_size) * 0.5,
            &self.label,
        );
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos
    }
}
