use apricot::{
    app::App,
    font::{Font, FontId},
    rectangle::Rectangle,
};
use hecs::{Entity, World};
use nalgebra_glm::{vec2, Vec2};

use crate::{
    astro::epoch::EphemerisTime,
    components::{
        body::Parent,
        factory::{cost_status, CostLine, Factory},
        parts::{PartDef, PartRegistry},
    },
    container,
    ui::{
        container::Container,
        hrule::HRule,
        label::Label,
        modal::Modal,
        scroll_container::ScrollContainer,
        style::STYLE,
        text_button::TextButton,
        widget::{recv_msgs, Widget},
    },
};

const WIDTH: f32 = 280.0;

pub struct FabricatorUi {
    modal: Modal<FabricatorMessages>,
    fabricator: Option<Entity>,
}

#[derive(Clone, Debug)]
enum FabricatorMessages {
    Build(u64), // part id, card-local
    Close,
}

pub struct FabricatorAction {
    pub fabricator: Entity,
    pub part_id: u64,
}

impl FabricatorUi {
    pub fn new() -> Self {
        Self {
            modal: Modal::new(Box::new(container![])),
            fabricator: None,
        }
    }

    pub fn update(
        &mut self,
        world: &World,
        registry: &PartRegistry,
        t: EphemerisTime,
        app: &App,
    ) -> Option<FabricatorAction> {
        for msg in recv_msgs(app, &mut self.modal) {
            println!("{msg:?}");
            match msg {
                FabricatorMessages::Build(part_id) => {
                    let Some(fabricator) = self.fabricator else {
                        panic!("wha!")
                    };
                    self.modal.set_shown(false);
                    return Some(FabricatorAction {
                        fabricator,
                        part_id,
                    });
                }
                FabricatorMessages::Close => {
                    self.modal.set_shown(false);
                }
            }
        }

        None
    }

    pub fn render(&self, app: &App) {
        self.modal.render(app);
    }

    pub fn show(
        &mut self,
        world: &World,
        fabricator: Entity,
        registry: &PartRegistry,
        t: EphemerisTime,
        app: &App,
    ) {
        let font = app.renderer.get_font_id_from_name("font").unwrap();
        let font_big: FontId = app.renderer.get_font_id_from_name("font-big").unwrap();

        self.fabricator = Some(fabricator);

        let station = world.get::<&Parent>(fabricator).unwrap().id;
        let pending = world
            .get::<&Factory>(fabricator)
            .ok()
            .and_then(|f| f.pending_job);

        let mut parts: Vec<&PartDef> = registry.all().collect();
        parts.sort_by(|a, b| a.name.cmp(&b.name));

        let mut cards: Vec<Box<dyn Widget<FabricatorMessages>>> = Vec::new();
        for part in parts {
            let lines = cost_status(world, station, &part.cost, t);
            cards.push(Box::new(
                self.build_card(part, &lines, registry, pending, app),
            ));
        }

        const CARD_W: f32 = 300.0;
        const HEIGHT: f32 = 400.0;

        self.modal = Modal::new(Box::new(ScrollContainer::new(
            vec2(CARD_W + 16.0, HEIGHT),
            Box::new(Container::new(cards)),
        )))
        .shown(true);
        self.modal.reposition(app);
    }

    fn build_card(
        &self,
        part: &PartDef,
        lines: &[CostLine],
        registry: &PartRegistry,
        pending: Option<u64>,
        app: &App,
    ) -> Container<FabricatorMessages> {
        let font = app.renderer.get_font_id_from_name("font").unwrap();
        let font_bold = app
            .renderer
            .get_font_id_from_name("font-small-bold")
            .unwrap();

        let id = part.id_hash();
        let affordable = lines.iter().all(|l| l.have >= l.need);
        let queued = pending == Some(id);

        const INNER_W: f32 = 280.0;

        let f = app.renderer.get_font_from_id(font).unwrap();

        let desc_lines: Vec<Box<dyn Widget<FabricatorMessages>>> = wrap(&part.desc, INNER_W, &f)
            .into_iter()
            .map(|l| {
                Box::new(Label::new(l).font(font, app).color(STYLE.text_primary))
                    as Box<dyn Widget<_>>
            })
            .collect();

        let mut widgets: Vec<Box<dyn Widget<FabricatorMessages>>> = vec![
            Box::new(
                Label::new(part.name.clone())
                    .font(font_bold, app)
                    .color(STYLE.text_primary),
            ),
            Box::new(Container::new(desc_lines).padding(Vec2::zeros()).gap(0.0)),
            Box::new(HRule::new(STYLE.border_primary, 1.0, INNER_W)),
        ];

        for line in lines {
            let text = match line.kind {
                crate::components::factory::CostKind::Part(part_id) => {
                    let name = registry
                        .get(part_id)
                        .map(|p| p.name.as_str())
                        .unwrap_or("???");
                    format!("{:.0}x {} (have {:.0})", name, line.need, line.have)
                }
                crate::components::factory::CostKind::Resource(r) => {
                    format!(
                        "{:.0} kg {} (have {:.0})",
                        line.need,
                        r.long_name(),
                        line.have
                    )
                }
            };
            widgets.push(Box::new(Label::new(text).font(font, app).color(
                if line.have >= line.need {
                    STYLE.text_primary
                } else {
                    STYLE.text_disabled
                },
            )));
        }

        widgets.push(Box::new(
            Label::new(format!("{:.0} kWh", part.cost.energy_kwh))
                .font(font, app)
                .color(STYLE.text_primary),
        ));

        widgets.push(Box::new(
            TextButton::new(
                Rectangle::new(0.0, 0.0, 280.0, 30.0),
                if queued { "QUEUED" } else { "BUILD" },
            )
            .use_style(&STYLE)
            .active(affordable && !queued)
            .on_click(FabricatorMessages::Build(id)),
        ));

        Container::new(widgets)
            .fixed_width(vec2(300.0, 0.0))
            .background_color(STYLE.bg_primary)
            .border(STYLE.border_primary, 1.0)
    }

    pub fn is_shown(&self) -> bool {
        self.modal.is_shown()
    }
}

fn wrap(text: &str, max_w: f32, font: &Font) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let candidate = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if font.measure(&candidate).x <= max_w || cur.is_empty() {
            cur = candidate;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string()
        }
    }
    if !cur.is_empty() {
        lines.push(cur)
    }
    lines
}
