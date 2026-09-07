use std::{cell::Cell, rc::Rc};

use apricot::{app::App, font::FontId, rectangle::Rectangle};
use nalgebra_glm::{vec2, vec4, Vec2, Vec4};

use crate::ui::{msg::MsgQueue, style::Style, widget::Widget};

pub struct Toggle<Msg> {
    rect: Rectangle,
    text_size: Vec2,
    label: String,
    state: Rc<Cell<bool>>, // what the switch shows
    active: bool,
    active_src: Option<Rc<Cell<bool>>>, // whether it accepts clicks
    on_toggle: Option<Msg>,
    font_id: Option<FontId>,
    text_color: Vec4,
    on_color: Vec4,
    off_color: Vec4,
    on_border: Vec4,
    off_border: Vec4,
    hovered: bool,
}

impl<Msg> Toggle<Msg> {
    const TRACK_W: f32 = 40.0;
    const TRACK_H: f32 = 18.0;

    pub fn new(label: impl Into<String>) -> Self {
        Self {
            rect: Rectangle {
                pos: Vec2::zeros(),
                size: Vec2::zeros(),
            },
            text_size: Vec2::zeros(),
            label: label.into(),
            hovered: false,
            state: Rc::new(Cell::new(false)),
            active: false,
            active_src: None,
            on_toggle: None,
            font_id: None,
            text_color: vec4(0.0, 0.0, 0.0, 0.0),
            on_color: vec4(0.0, 0.0, 0.0, 0.0),
            off_color: vec4(0.0, 0.0, 0.0, 0.0),
            on_border: vec4(0.0, 0.0, 0.0, 0.0),
            off_border: vec4(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn bind(mut self, state: Rc<Cell<bool>>) -> Self {
        self.state = state.clone();
        self
    }

    pub fn font(mut self, font_id: FontId, app: &App) -> Self {
        self.font_id = Some(font_id);
        let font = app.renderer.get_font_from_id(font_id).unwrap();
        self.text_size = font.measure(&self.label);
        self.rect.size.x = Self::TRACK_W + 8.0 + self.text_size.x;
        self.rect.size.y = Self::TRACK_H.max(self.text_size.y);
        self
    }

    pub fn use_style(mut self, style: &Style) -> Self {
        self.text_color = style.text_primary;
        self.on_color = style.btn_accent_bg;
        self.off_color = style.btn_active_bg;
        self.on_border = style.btn_accent_border;
        self.off_border = style.btn_active_border;
        self
    }

    pub fn on_toggle(mut self, msg: Msg) -> Self {
        self.on_toggle = Some(msg);
        self
    }

    pub fn bound_active(mut self, active_src: Rc<Cell<bool>>) -> Self {
        self.active_src = Some(active_src);
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Toggle<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        if let Some(src) = &self.active_src {
            self.active = src.get();
        }

        if self.active {
            self.hovered = app.mouse_over(&self.rect);
            if self.hovered && !app.is_click_consumed() && app.mouse_left_clicked {
                if let Some(msg) = &self.on_toggle {
                    msgq.push(msg.clone());
                    app.consume_click();
                }
            }
        }
    }

    fn render(&self, app: &App) {
        let on = self.state.get();

        // label on the left
        let text_pos =
            self.rect.pos + vec2(0.0, (self.rect.size.y - self.text_size.y).max(0.0) / 2.0);
        app.renderer.set_color(self.text_color);
        app.renderer.set_font(self.font_id.unwrap());
        app.renderer.draw_text(text_pos, &self.label);

        // switch track on the right
        let track = Rectangle::new(
            self.rect.pos.x + self.rect.size.x - Self::TRACK_W,
            self.rect.pos.y + (self.rect.size.y - Self::TRACK_H) * 0.5,
            Self::TRACK_W,
            Self::TRACK_H,
        );
        app.renderer
            .set_color(if on { self.on_color } else { self.off_color });
        app.renderer.fill_rect(track);

        // border
        let border_color = if on { self.on_border } else { self.off_border };
        app.renderer.set_color(border_color);
        app.renderer.draw_rect(track, 1.0);

        // knob
        let knob_x = if on {
            track.pos.x + Self::TRACK_W - Self::TRACK_H
        } else {
            track.pos.x
        };
        app.renderer.set_color(self.text_color);
        app.renderer.fill_rect(Rectangle::new(
            knob_x,
            track.pos.y,
            Self::TRACK_H,
            Self::TRACK_H,
        ));
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }
    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos;
    }
}
