use std::{cell::Cell, marker::PhantomData, rc::Rc};

use apricot::{app::App, rectangle::Rectangle, render_core::TextureId};
use nalgebra_glm::{vec2, vec4, Vec2, Vec4};

use crate::ui::{msg::MsgQueue, widget::Widget};

pub struct PorkchopPicker<Msg> {
    /// The rectangle defining the texture's position and size
    rect: Rectangle,
    /// The texture ID to render
    texture_id: TextureId,
    cols: usize,
    rows: usize,
    selected: Rc<Cell<(usize, usize)>>,
    optimum: Rc<Cell<(usize, usize)>>,
    dragging: bool,
    _msg: PhantomData<Msg>,
}

impl<Msg> PorkchopPicker<Msg> {
    /// Creates a new porkchop plot picker
    pub fn new(
        size: Vec2,
        texture_id: TextureId,
        cols: usize,
        rows: usize,
        selected: Rc<Cell<(usize, usize)>>,
        optimum: Rc<Cell<(usize, usize)>>,
    ) -> Self {
        Self {
            rect: Rectangle {
                pos: Vec2::zeros(),
                size,
            },
            texture_id,
            cols,
            rows,
            selected,
            optimum,
            dragging: false,
            _msg: PhantomData,
        }
    }

    fn cell_center(&self, i: usize, j: usize) -> Vec2 {
        vec2(
            self.rect.pos.x + (i as f32 + 0.5) / self.cols as f32 * self.rect.size.x,
            self.rect.pos.y + (j as f32 + 0.5) / self.rows as f32 * self.rect.size.y,
        )
    }

    fn cell_at(&self, pos: Vec2) -> (usize, usize) {
        let u = ((pos.x - self.rect.pos.x) / self.rect.size.x).clamp(0.0, 1.0);
        let v = ((pos.y - self.rect.pos.y) / self.rect.size.y).clamp(0.0, 1.0);
        let i = ((u * self.cols as f32) as usize).min(self.cols - 1);
        let j = ((v * self.rows as f32) as usize).min(self.rows - 1);
        (i, j)
    }

    fn draw_crosshair(&self, app: &App, c: Vec2, color: Vec4, thickness: f32) {
        const ARM: f32 = 10.0; // arm length
        const GAP: f32 = 4.0; // hole radius

        let t = thickness;
        app.renderer.set_color(color);
        app.renderer
            .fill_rect(Rectangle::new(c.x - GAP - ARM, c.y - t * 0.5, ARM, t)); // left
        app.renderer
            .fill_rect(Rectangle::new(c.x + GAP, c.y - t * 0.5, ARM, t)); // right
        app.renderer
            .fill_rect(Rectangle::new(c.x - t * 0.5, c.y - GAP - ARM, t, ARM)); // up
        app.renderer
            .fill_rect(Rectangle::new(c.x - t * 0.5, c.y + GAP, t, ARM)); // down
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for PorkchopPicker<Msg> {
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
            self.selected.set(self.cell_at(app.mouse_pos))
        }
    }

    /// Render the texture to the screen
    fn render(&self, app: &App) {
        app.renderer.copy_texture(
            self.rect,
            self.texture_id,
            Rectangle {
                pos: vec2(0.0, 0.0),
                size: vec2(self.cols as f32, self.rows as f32),
            },
            &vec4(1.0, 1.0, 1.0, 1.0),
        );

        let (i, j) = self.optimum.get();
        let optimum_c = self.cell_center(i, j);
        let diamond = app
            .renderer
            .get_mesh_id_from_name("square-outline")
            .unwrap();
        app.renderer.set_color(vec4(0.0, 0.0, 0.0, 0.7));
        app.renderer.fill_polygon(diamond, optimum_c, 7.0, 0.0);

        let (i, j) = self.selected.get();
        let c = self.cell_center(i, j);
        self.draw_crosshair(app, c, vec4(1.0, 1.0, 1.0, 1.0), 3.0);
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos
    }
}
