use std::{
    cell::{Cell, RefCell},
    f32::consts::FRAC_PI_2,
    rc::Rc,
};

use apricot::{app::App, rectangle::Rectangle, render_core::MeshId};
use nalgebra_glm::{vec2, vec4, Vec2, Vec4};

use crate::{
    astro::epoch::EphemerisTime,
    scenes::events::Event,
    ui::{msg::MsgQueue, style::Style, widget::Widget},
};

pub struct Timeline {
    baseline_color: Vec4,
    now_color: Vec4,
    rect: Rectangle,
    start: Rc<Cell<EphemerisTime>>,
    span_years: f64,
    marks: Rc<RefCell<Vec<TimelineMark>>>,
    hovered: Option<usize>,
}

impl Timeline {
    pub const HEIGHT: f32 = 80.0;

    pub fn new(
        start: Rc<Cell<EphemerisTime>>,
        thickness: f32,
        marks: Rc<RefCell<Vec<TimelineMark>>>,
    ) -> Self {
        Self {
            baseline_color: vec4(0.0, 0.0, 0.0, 1.0),
            now_color: vec4(0.0, 0.0, 0.0, 1.0),
            rect: Rectangle::new(0.0, 0.0, thickness, Self::HEIGHT),
            start,
            span_years: 1.0 / 12.0,
            marks,
            hovered: None,
        }
    }

    pub fn use_style(mut self, style: &Style) -> Self {
        self.baseline_color = style.border_primary;
        self.now_color = style.text_primary;
        self
    }
}

#[derive(Clone)]
pub struct TimelineMark {
    pub t: EphemerisTime,
    pub kind: MarkKind,
    pub craft_name: String,
}

#[derive(Clone, Copy)]
pub enum MarkKind {
    Burn,
    SoiChange,
    Launch,
    Land,
    FactoryComplete,
    Background,
}

impl MarkKind {
    pub fn from_event(event: &Event) -> Option<Self> {
        match event {
            Event::SoiChange { .. } => Some(MarkKind::SoiChange),
            Event::Burn { .. } => Some(MarkKind::Burn),
            Event::Launch { .. } => Some(MarkKind::Launch),
            Event::Land { .. } => Some(MarkKind::Land),
            Event::FactoryComplete { .. } => Some(MarkKind::FactoryComplete),
            Event::Background => Some(MarkKind::Background),

            Event::CompleteCommand { .. } => None,
        }
    }

    fn color(&self) -> Vec4 {
        const MARK_L: f32 = 0.8;
        const MARK_C: f32 = 0.13;
        match *self {
            MarkKind::Burn => oklch(MARK_L, MARK_C, 55.0, 1.0),
            MarkKind::Launch => oklch(MARK_L, MARK_C, 130.0, 1.0),
            MarkKind::Land => oklch(MARK_L, MARK_C, 195.0, 1.0),
            MarkKind::SoiChange => oklch(MARK_L, MARK_C, 265.0, 1.0),
            MarkKind::FactoryComplete => oklch(MARK_L, MARK_C, 330.0, 1.0),
            MarkKind::Background => oklch(0.0, MARK_C, 55.0, 1.0),
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            MarkKind::Burn => "Burn",
            MarkKind::SoiChange => "SOI Crossing",
            MarkKind::Launch => "Launch",
            MarkKind::Land => "Land",
            MarkKind::FactoryComplete => "Part Complete",
            MarkKind::Background => "Next Turn",
        }
    }

    fn shape(&self, app: &App) -> (MeshId, f32) {
        match self {
            MarkKind::Burn => (
                // cause its pointy?
                app.renderer
                    .get_mesh_id_from_name("square-outline")
                    .unwrap(),
                0.0,
            ),
            MarkKind::SoiChange => (
                // special shape
                app.renderer
                    .get_mesh_id_from_name("hexagon-outline")
                    .unwrap(),
                0.0,
            ),
            MarkKind::Launch => (
                // triangle pointing up
                app.renderer
                    .get_mesh_id_from_name("triangle-outline")
                    .unwrap(),
                FRAC_PI_2,
            ),
            MarkKind::Land => (
                // triangle pointing down
                app.renderer
                    .get_mesh_id_from_name("triangle-outline")
                    .unwrap(),
                -FRAC_PI_2,
            ),
            MarkKind::FactoryComplete => (
                // like a box
                app.renderer
                    .get_mesh_id_from_name("square-outline")
                    .unwrap(),
                45.0f32.to_radians(),
            ),
            MarkKind::Background => (
                // special shape
                app.renderer
                    .get_mesh_id_from_name("pentagon-outline")
                    .unwrap(),
                0.0,
            ),
        }
    }
}

pub fn oklch(l: f32, c: f32, h_deg: f32, alpha: f32) -> Vec4 {
    let h = h_deg.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());

    // OKLab to LMS
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_18 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (lc, mc, sc) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    // LMS to linear sRGB
    let r = 4.076_741_7 * lc - 3.307_711_6 * mc + 0.230_969_94 * sc;
    let g = -1.268_438 * lc + 2.609_757_4 * mc - 0.341_319_38 * sc;
    let bl = -0.0041960863 * lc - 0.703_418_6 * mc + 1.707_614_7 * sc;

    vec4(encode(r), encode(g), encode(bl), alpha)
}

fn encode(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Timeline {
    fn update(&mut self, app: &App, _msgq: &mut MsgQueue<Msg>) {
        const MARK_R: f32 = 8.5;
        let cy = self.baseline_y();
        let shape_cy = cy - Self::EVENT_TICK_HEIGHT - 13.0;

        let m = app.mouse_pos;

        self.hovered = self
            .marks
            .borrow()
            .iter()
            .enumerate()
            .filter_map(|(i, mark)| {
                let x = self.x_for(mark.t)?;
                let dx = (m.x - x).abs();
                (dx <= MARK_R && m.y >= shape_cy - MARK_R && m.y <= cy).then_some((i, dx))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
    }

    fn render(&self, app: &App) {
        let timeline_baseline: f32 = self.baseline_y();

        let timeline = Rectangle {
            pos: vec2(self.rect.pos.x, timeline_baseline),
            size: vec2(self.rect.size.x, 1.0),
        };

        app.renderer.set_color(self.baseline_color);
        app.renderer.fill_rect(timeline);

        // Draw the ticks
        let step = EphemerisTime::from_days(7.0);
        let mut et = self.start.get().ceil_to(step);
        while let Some(x) = self.x_for(et) {
            const TICK_HEIGHT: f32 = 13.0;
            let tick = Rectangle {
                pos: vec2(x, timeline_baseline),
                size: vec2(1.0, TICK_HEIGHT),
            };
            app.renderer.set_color(self.baseline_color);
            app.renderer.fill_rect(tick);

            let label = format!("{} {}", et.short_month_name(), et.day_of_month());
            let font_id = app.renderer.get_current_font_id().unwrap();
            let font = app.renderer.get_font_from_id(font_id).unwrap();
            let width = font.measure(&label).x;

            app.renderer.set_color(self.now_color);
            app.renderer.draw_text(
                vec2(x - width * 0.5, timeline_baseline + TICK_HEIGHT + 6.0),
                &label,
            );
            et += step;
        }

        // Draw events
        for timeline_event in self.marks.borrow().iter() {
            let Some(x) = self.x_for(timeline_event.t) else {
                continue;
            };
            let event_tick = Rectangle {
                pos: vec2(x, timeline_baseline - Self::EVENT_TICK_HEIGHT),
                size: vec2(1.0, Self::EVENT_TICK_HEIGHT),
            };

            let color = timeline_event.kind.color();
            app.renderer.set_color(color);
            app.renderer.fill_rect(event_tick);
            let (mesh_id, rotation) = timeline_event.kind.shape(app);
            app.renderer.fill_polygon(
                mesh_id,
                vec2(event_tick.pos.x + 1.0, event_tick.pos.y - 13.0),
                8.5,
                rotation,
            );
        }

        // Draw the hovered event
        if let Some(hovered) = self.hovered {
            let timeline_event = &self.marks.borrow()[hovered];

            if let Some(x) = self.x_for(timeline_event.t) {
                let event_tick = Rectangle {
                    pos: vec2(x, timeline_baseline - Self::EVENT_TICK_HEIGHT),
                    size: vec2(1.0, Self::EVENT_TICK_HEIGHT),
                };

                app.renderer.set_color(self.now_color);
                app.renderer.draw_text(
                    vec2(event_tick.pos.x + 15.0, event_tick.pos.y - 1.0),
                    &timeline_event.craft_name,
                );
                let color = timeline_event.kind.color();
                let event_label = String::from(timeline_event.kind.describe());
                app.renderer.set_color(color);
                app.renderer.draw_text(
                    vec2(event_tick.pos.x + 15.0, event_tick.pos.y - 20.0),
                    &event_label,
                );
            }
        }

        // Draw the "now" cursor
        let now_x = self.x_for(self.start.get()).unwrap();
        let now = Rectangle {
            pos: vec2(now_x, self.rect.pos.y),
            size: vec2(2.0, self.rect.size.y),
        };
        app.renderer.set_color(self.now_color);
        app.renderer.fill_rect(now);
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos;
    }
}

impl Timeline {
    const EVENT_TICK_HEIGHT: f32 = 20.0;

    fn x_for(&self, t: EphemerisTime) -> Option<f32> {
        let frac = (t.as_years() - self.start.get().as_years()) / self.span_years;
        (0.0..=1.0)
            .contains(&frac)
            .then(|| self.rect.pos.x + frac as f32 * self.rect.size.x)
    }

    fn baseline_y(&self) -> f32 {
        self.rect.pos.y + self.rect.size.y * 0.5
    }
}
