use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{vec2, vec4, Vec2, Vec4};

use crate::ui::{msg::MsgQueue, style::Style, widget::Widget};

pub struct Dropdown<Msg> {
    rect: Rectangle,
    label: String,
    options: Vec<(String, Msg)>,
    open: bool,
    selected: Option<usize>,

    // Colors
    background_color: Vec4,
    hover_color: Vec4,
    border_color: Vec4,
    text_color: Vec4,
}

impl<Msg: Clone + 'static> Dropdown<Msg> {
    pub fn new(
        label: impl Into<String>,
        size: Vec2,
        options: Vec<(impl Into<String>, Msg)>,
    ) -> Self {
        Self {
            rect: Rectangle::new(0.0, 0.0, size.x, size.y),
            label: label.into(),
            options: options.into_iter().map(|(s, m)| (s.into(), m)).collect(),
            open: false,
            selected: None,
            background_color: vec4(1.0, 0.0, 1.0, 1.0),
            hover_color: vec4(1.0, 0.0, 1.0, 1.0),
            border_color: vec4(1.0, 0.0, 1.0, 1.0),
            text_color: vec4(1.0, 0.0, 1.0, 1.0),
        }
    }

    pub fn selected(mut self, idx: Option<usize>) -> Self {
        self.selected = idx;
        self
    }

    pub fn use_style(mut self, style: &Style) -> Self {
        self.background_color = style.bg_primary;
        self.hover_color = style.bg_hover;
        self.border_color = style.border_primary;
        self.text_color = style.text_primary;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Dropdown<Msg> {
    fn overlay_update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        if !self.open {
            return;
        }
        if app.mouse_left_clicked && !app.is_click_consumed() {
            for (i, (_, msg)) in self.options.iter().enumerate() {
                let option_rect = Rectangle {
                    pos: vec2(self.rect.pos.x, self.rect.pos.y + 28.0 + i as f32 * 28.0),
                    size: vec2(self.rect.size.x, 28.0),
                };
                if option_rect.contains_point(&app.mouse_pos) {
                    self.selected = Some(i);
                    self.open = false;
                    msgq.push(msg.clone());
                    app.consume_click();
                    return;
                }
            }
            // Click within menu area but not on an option — consume to block passthrough
            let menu_rect = Rectangle {
                pos: vec2(self.rect.pos.x, self.rect.pos.y + 28.0),
                size: vec2(self.rect.size.x, self.options.len() as f32 * 28.0),
            };
            if menu_rect.contains_point(&app.mouse_pos) {
                app.consume_click();
            } else {
                // Click outside the whole dropdown — close it, let click reach its target
                self.open = false;
            }
        }
    }

    fn update(&mut self, app: &App, _msgq: &mut MsgQueue<Msg>) {
        let header_rect = Rectangle {
            pos: self.rect.pos,
            size: vec2(self.rect.size.x, 28.0),
        };
        if app.mouse_left_clicked && !app.is_click_consumed() && header_rect.contains_point(&app.mouse_pos) {
            self.open = !self.open;
            app.consume_click();
        }
    }

    fn render(&self, app: &App) {
        // Header
        let selected_label = self
            .selected
            .and_then(|i| self.options.get(i))
            .map(|(s, _)| s.as_str())
            .unwrap_or(&self.label);

        app.renderer.set_color(self.background_color);
        app.renderer.fill_rect(Rectangle {
            pos: self.rect.pos,
            size: vec2(self.rect.size.x, 28.0),
        });
        app.renderer.set_color(self.border_color);
        app.renderer.draw_rect(
            Rectangle {
                pos: self.rect.pos,
                size: vec2(self.rect.size.x, 28.0),
            },
            1.0,
        );

        let font = app.renderer.get_current_font().unwrap();
        app.renderer.set_color(self.text_color);
        font.draw(
            self.rect.pos + vec2(6.0, 6.0),
            &format!("{} ▾", selected_label),
            &app.renderer,
        );

        // Options: push to a higher draw layer so they render on top of all sibling widgets
        if self.open {
            app.renderer.set_draw_layer(10);
            for (i, (label, _)) in self.options.iter().enumerate() {
                let option_rect = Rectangle {
                    pos: vec2(self.rect.pos.x, self.rect.pos.y + 28.0 + i as f32 * 28.0),
                    size: vec2(self.rect.size.x, 28.0),
                };
                let hovered = option_rect.contains_point(&app.mouse_pos);
                app.renderer.set_color(if hovered {
                    self.hover_color
                } else {
                    self.background_color
                });
                app.renderer.fill_rect(option_rect);
                app.renderer.set_color(self.border_color);
                app.renderer.draw_rect(option_rect, 1.0);
                app.renderer.set_color(self.text_color);
                font.draw(option_rect.pos + vec2(6.0, 6.0), label, &app.renderer);
            }
            app.renderer.set_draw_layer(0);
        }
    }

    fn size(&self) -> Vec2 {
        // Only report header height to layout, dropdown overlaps when open
        vec2(self.rect.size.x, 28.0)
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos;
    }
}
