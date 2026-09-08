use std::{cell::Cell, marker::PhantomData, rc::Rc};

use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{vec2, vec4, Vec2, Vec4};

use crate::ui::{msg::MsgQueue, style::Style, widget::Widget};

pub struct Slider<Msg> {
    rect: Rectangle,
    value: Rc<Cell<f32>>, // normalized 0..1
    dragging: bool,
    track_color: Vec4,
    knob_color: Vec4,
    _msg: PhantomData<Msg>,
}

impl<Msg> Slider<Msg> {
    pub fn new(size: Vec2, value: Rc<Cell<f32>>) -> Self {
        Self {
            rect: Rectangle {
                pos: vec2(0.0, 0.0),
                size,
            },
            value,
            dragging: false,
            track_color: vec4(1.0, 1.0, 0.0, 1.0),
            knob_color: vec4(1.0, 1.0, 0.0, 1.0),
            _msg: PhantomData,
        }
    }

    pub fn use_style(mut self, style: &Style) -> Self {
        self.track_color = style.border_primary;
        self.knob_color = style.border_primary;
        self
    }

    fn value_at(&self, pos: Vec2) -> f32 {
        ((pos.x - self.rect.pos.x) / self.rect.size.x).clamp(0.0, 1.0)
    }

    fn knob_x(&self) -> f32 {
        self.rect.pos.x + self.value.get() * self.rect.size.x
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Slider<Msg> {
    fn update(&mut self, app: &App, _msgq: &mut MsgQueue<Msg>) {
        if !app.is_click_consumed() {
            if (app.mouse_left_dragging || app.mouse_left_clicked) && app.mouse_over(&self.rect) {
                self.dragging = true;
            }
            if !app.mouse_left_dragging {
                self.dragging = false;
            }
        }
        if self.dragging {
            self.value.set(self.value_at(app.mouse_pos));
        }
    }

    fn render(&self, app: &App) {
        const TRACK: f32 = 4.0;
        let cy = self.rect.pos.y + self.rect.size.y * 0.5;

        app.renderer.set_color(self.track_color);
        app.renderer.fill_rect(Rectangle::new(
            self.rect.pos.x,
            cy - TRACK * 0.5,
            self.rect.size.x,
            TRACK,
        ));

        app.renderer.set_color(self.knob_color);
        app.renderer
            .fill_rect(Rectangle::new(self.knob_x() - 3.0, cy - 8.0, 6.0, 16.0));
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }
    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos
    }
}
