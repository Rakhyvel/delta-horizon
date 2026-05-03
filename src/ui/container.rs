use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::{vec2, Vec2};

/// How widgets are stacked
pub enum Flow {
    Vertical,
    Horizontal,
}

/// How widgets are position on the flow-axis
pub enum Align {
    // left or top
    Start,
    Center,
    // right or bottom
    End,
}

pub struct Container<Msg> {
    rect: Rectangle,
    children: Vec<Box<dyn Widget<Msg>>>,
    flow: Flow,
    /// Alignment on the perpendicular axis
    cross_align: Align,
    fixed_width: bool,
    fixed_height: bool,
    padding: Vec2,
    background: Option<nalgebra_glm::Vec4>,
    border: Option<(nalgebra_glm::Vec4, f32)>, // color, width
}

impl<Msg: Clone + 'static> Container<Msg> {
    pub fn new(children: Vec<Box<dyn Widget<Msg>>>) -> Self {
        let mut retval = Self {
            rect: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            children,
            flow: Flow::Vertical,
            cross_align: Align::Start,
            fixed_width: false,
            fixed_height: false,
            padding: vec2(8.0, 8.0),
            background: None,
            border: None,
        };
        retval.layout(retval.rect.pos);
        retval
    }

    pub fn at(mut self, pos: Vec2) -> Self {
        self.rect.pos = pos;
        self.layout(self.rect.pos);
        self
    }

    pub fn flow(mut self, flow: Flow) -> Self {
        self.flow = flow;
        self.layout(self.rect.pos);
        self
    }

    pub fn cross_align(mut self, cross_align: Align) -> Self {
        self.cross_align = cross_align;
        self.layout(self.rect.pos);
        self
    }

    pub fn fixed_size(mut self, size: Vec2) -> Self {
        self.rect.size = size;
        self.fixed_height = true;
        self.fixed_width = true;
        self.layout(self.rect.pos);
        self
    }

    pub fn padding(mut self, padding: Vec2) -> Self {
        self.padding = padding;
        self.layout(self.rect.pos);
        self
    }

    pub fn background(mut self, color: nalgebra_glm::Vec4) -> Self {
        self.background = Some(color);
        self
    }

    pub fn border(mut self, color: nalgebra_glm::Vec4, width: f32) -> Self {
        self.border = Some((color, width));
        self
    }
}

#[macro_export]
macro_rules! container {
    ($($widget:expr),* $(,)?) => {
        Container::new(vec![$(Box::new($widget)),*])
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Container<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        for child in self.children.iter_mut() {
            child.as_mut().update(app, msgq);
        }
    }

    fn render(&self, app: &App) {
        // Draw background
        if let Some(color) = self.background {
            app.renderer.set_color(color);
            app.renderer.fill_rect(self.rect);
        }

        for child in self.children.iter() {
            child.as_ref().render(app);
        }
    }

    fn size(&self) -> Vec2 {
        self.rect.size
    }

    fn layout(&mut self, pos: Vec2) {
        self.rect.pos = pos;

        // Offset children by padding
        let inner_pos = pos + self.padding;

        // Collect child sizes
        let child_sizes: Vec<Vec2> = self
            .children
            .iter_mut()
            .map(|child| {
                child.layout(Vec2::zeros());
                child.size()
            })
            .collect();

        let max_content_size = child_sizes
            .iter()
            .fold(Vec2::zeros(), |acc, s| nalgebra_glm::max2(&acc, s));
        let additive_content_size = child_sizes.iter().fold(Vec2::zeros(), |acc, s| acc + *s);

        // Compute spacer
        let mut spacer = if self.fixed_width || self.fixed_height {
            if self.children.is_empty() {
                Vec2::zeros()
            } else {
                (self.rect.size - additive_content_size) / (self.children.len() as f32 + 1.0)
            }
        } else {
            Vec2::zeros() // content-sized container has no spare space to distribute
        };
        match self.flow {
            Flow::Vertical => spacer.x = 0.0,
            Flow::Horizontal => spacer.y = 0.0,
        }
        if spacer.x < 0.0 {
            spacer.x = 0.0;
        }
        if spacer.y < 0.0 {
            spacer.y = 0.0;
        }

        // Second pass to place children
        let mut main_offset = match self.flow {
            Flow::Vertical => spacer.y,
            Flow::Horizontal => spacer.x,
        };
        let mut working_size = Vec2::zeros();

        for (child, child_size) in self.children.iter_mut().zip(child_sizes.iter()) {
            // Cross axis: how to position perpendicular to flow
            let cross_size_available = match self.flow {
                Flow::Vertical => {
                    if self.fixed_width {
                        self.rect.size.x
                    } else {
                        max_content_size.x
                    }
                }
                Flow::Horizontal => {
                    if self.fixed_height {
                        self.rect.size.y
                    } else {
                        max_content_size.y
                    }
                }
            };
            let cross_child_size = match self.flow {
                Flow::Vertical => child_size.x,
                Flow::Horizontal => child_size.y,
            };
            let cross_offset = match self.cross_align {
                Align::Start => 0.0,
                Align::Center => cross_size_available / 2.0 - cross_child_size / 2.0,
                Align::End => cross_size_available - cross_child_size,
            };

            // Compute child position from main + cross offsets
            let child_pos = match self.flow {
                Flow::Vertical => vec2(inner_pos.x + cross_offset, inner_pos.y + main_offset),
                Flow::Horizontal => vec2(inner_pos.x + main_offset, inner_pos.y + cross_offset),
            };
            child.layout(child_pos);

            // Advance main axis and accumulate working size
            match self.flow {
                Flow::Vertical => {
                    main_offset += spacer.y + child_size.y;
                    working_size.x = max_content_size.x;
                    working_size.y += spacer.y + child_size.y;
                }
                Flow::Horizontal => {
                    main_offset += spacer.x + child_size.x;
                    working_size.x += spacer.x + child_size.x;
                    working_size.y = max_content_size.y;
                }
            }
        }

        if !self.fixed_width {
            self.rect.size.x = working_size.x + self.padding.x * 2.0;
        }
        if !self.fixed_height {
            self.rect.size.y = working_size.y + self.padding.y * 2.0;
        }
    }
}
