use crate::{
    astro::{
        epoch::EphemerisTime,
        escape::{plan_escape, EscapePlan},
        landing::{plan_landing, LandingPlan},
        launch::{plan_launch, LaunchPlan},
        state::State,
        transfer::{plan_flyby, plan_transfer, FlybyPlan, TransferObjective, TransferPlan},
    },
    components::{
        body::{Body, Parent, SceneObject},
        craft::{Command, Craft, Landed},
    },
    ui::{container::Container, dropdown::Dropdown, hrule::HRule, style::STYLE},
};
use apricot::{app::App, font::FontId, rectangle::Rectangle};
use hecs::{Entity, World};
use nalgebra_glm::{vec2, vec4};

use crate::{
    container,
    ui::{
        container::{Align, Flow},
        label::Label,
        modal::Modal,
        text_button::TextButton,
        widget::{recv_msgs, Widget},
    },
};

const WIDTH: f32 = 280.0;

pub struct ManeuverModal {
    modal: Modal<ManeuverMessages>,
    craft: Option<Entity>,

    selected_kind: Option<ManeuverKind>,
    selected_destination: Option<Entity>,
    selected_date: EphemerisTime,
    computed_plan: Option<ManeuverResult>,
}

#[derive(Clone, Debug)]
enum ManeuverMessages {
    SelectKind(ManeuverKind),
    SelectDestination(Entity),
    AdjustDate(f64),
    Confirm,
    Close,
}

#[derive(Clone, Debug, PartialEq)]
enum ManeuverKind {
    Transfer,
    Flyby,
    Escape,
    Land,
    Launch,
}

impl ManeuverKind {
    pub fn all() -> &'static [ManeuverKind] {
        &[
            ManeuverKind::Transfer,
            ManeuverKind::Flyby,
            ManeuverKind::Escape,
            ManeuverKind::Land,
            ManeuverKind::Launch,
        ]
    }

    pub fn available(&self, is_landed: bool, is_orbiting: bool) -> bool {
        match self {
            ManeuverKind::Transfer => is_orbiting,
            ManeuverKind::Flyby => is_orbiting,
            ManeuverKind::Escape => is_orbiting,
            ManeuverKind::Land => is_orbiting,
            ManeuverKind::Launch => is_landed,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ManeuverKind::Transfer => "Transfer",
            ManeuverKind::Flyby => "Flyby",
            ManeuverKind::Escape => "Escape",
            ManeuverKind::Land => "Land",
            ManeuverKind::Launch => "Launch",
        }
    }

    pub fn needs_destination(&self) -> bool {
        match self {
            ManeuverKind::Transfer => true,
            ManeuverKind::Flyby => true,
            ManeuverKind::Escape => false,
            ManeuverKind::Land => false,
            ManeuverKind::Launch => false,
        }
    }
}

pub enum ManeuverResult {
    Transfer { to: Entity, plan: TransferPlan },
    Flyby { to: Entity, plan: FlybyPlan },
    Escape { to: Entity, plan: EscapePlan },
    Land { plan: LandingPlan },
    Launch { plan: LaunchPlan },
}

impl ManeuverResult {
    pub fn total_dv(&self) -> f64 {
        match self {
            ManeuverResult::Transfer { plan, .. } => plan.transfer_dv + plan.circ_dv,
            ManeuverResult::Flyby { plan, .. } => plan.transfer_dv,
            ManeuverResult::Escape { plan, .. } => plan.escape_dv,
            ManeuverResult::Land { plan } => plan.deorbit_dv + plan.landing_dv,
            ManeuverResult::Launch { plan } => plan.launch_dv + plan.circ_dv,
        }
    }

    pub fn arrival_et(&self) -> EphemerisTime {
        match self {
            ManeuverResult::Transfer { plan, .. } => plan.circ_state.t,
            ManeuverResult::Flyby { plan, .. } => plan.flyby_state.t,
            ManeuverResult::Escape { plan, .. } => plan.grandparent_orbit.t,
            ManeuverResult::Land { plan } => plan.landing_burn.t,
            ManeuverResult::Launch { plan } => plan.circ_burn.t,
        }
    }

    pub fn can_afford(&self, craft_dv: f64) -> bool {
        self.total_dv() <= craft_dv
    }

    pub fn into_command(self) -> Command {
        match self {
            ManeuverResult::Transfer { to, plan } => Command::Transfer { to, plan },
            ManeuverResult::Flyby { to, plan } => Command::Flyby { to, plan },
            ManeuverResult::Escape { to, plan } => Command::Escape { to, plan },
            ManeuverResult::Land { plan } => Command::Land { plan },
            ManeuverResult::Launch { plan } => Command::Launch { plan },
        }
    }
}

impl ManeuverModal {
    pub fn new() -> Self {
        Self {
            modal: Modal::new(Box::new(container![])),
            craft: None,

            selected_kind: None,
            selected_destination: None,
            selected_date: EphemerisTime::epoch(),
            computed_plan: None,
        }
    }

    pub fn show(&mut self, craft: Entity, current_et: EphemerisTime, world: &World, app: &App) {
        self.craft = Some(craft);

        self.selected_kind = None;
        self.selected_destination = None;
        self.selected_date = current_et;
        self.computed_plan = None;

        self.rebuild(world, app);
        self.modal.set_shown(true);
    }

    pub fn is_shown(&self) -> bool {
        self.modal.is_shown()
    }

    pub fn update(
        &mut self,
        current_et: EphemerisTime,
        world: &World,
        app: &App,
    ) -> Option<ManeuverResult> {
        let craft = self.craft?;
        for msg in recv_msgs(app, &mut self.modal) {
            match msg {
                ManeuverMessages::SelectKind(maneuver_kind) => {
                    self.selected_kind = Some(maneuver_kind);
                    self.computed_plan = self.compute_plan(craft, world);
                    self.rebuild(world, app)
                }
                ManeuverMessages::SelectDestination(entity) => {
                    self.selected_destination = Some(entity);
                    self.computed_plan = self.compute_plan(craft, world);
                    self.rebuild(world, app)
                }
                ManeuverMessages::AdjustDate(day_offset) => {
                    self.selected_date += EphemerisTime::from_days(day_offset);
                    self.selected_date = self.selected_date.max(current_et);
                    self.computed_plan = self.compute_plan(craft, world);
                    self.rebuild(world, app)
                }
                ManeuverMessages::Close => {
                    self.modal.set_shown(false);
                }
                ManeuverMessages::Confirm => {
                    self.modal.set_shown(false);
                    return self.computed_plan.take();
                }
            }
        }
        None
    }

    pub fn render(&self, app: &App) {
        self.modal.render(app);
    }

    pub fn rebuild(&mut self, world: &World, app: &App) {
        let font = app.renderer.get_font_id_from_name("font").unwrap();
        let font_small_bold: FontId = app
            .renderer
            .get_font_id_from_name("font-small-bold")
            .unwrap();
        let font_big: FontId = app.renderer.get_font_id_from_name("font-big").unwrap();

        let mut sections: Vec<Box<dyn Widget<ManeuverMessages>>> = vec![
            Box::new(Label::new("PLAN MANEUVER:").font(font_big, app)),
            Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)),
        ];

        let craft_dv = world
            .get::<&Craft>(self.craft.unwrap())
            .unwrap()
            .total_remaining_dv();
        let is_landed = world.get::<&Landed>(self.craft.unwrap()).is_ok();
        let is_orbiting = world.get::<&State>(self.craft.unwrap()).is_ok();

        let kind_options: Vec<&ManeuverKind> = ManeuverKind::all()
            .iter()
            .filter(|k| k.available(is_landed, is_orbiting))
            .collect();
        let selected_kind_idx = self
            .selected_kind
            .as_ref()
            .and_then(|sk| kind_options.iter().position(|k| *k == sk));

        sections.push(Box::new(
            container!(
                Label::new("Maneuver kind:").font(font, app),
                Dropdown::new(
                    "",
                    vec2(WIDTH * 0.5, 40.0),
                    kind_options
                        .iter()
                        .map(|k| (k.label(), ManeuverMessages::SelectKind((*k).clone())))
                        .collect(),
                )
                .selected(selected_kind_idx)
                .use_style(&STYLE),
            )
            .flow(Flow::Horizontal)
            .cross_align(Align::Center)
            .padding(vec2(12.0, 0.0)),
        ));

        if let Some(kind) = &self.selected_kind {
            if kind.needs_destination() {
                let destinations = self.get_destinations(self.craft.unwrap(), world);
                let selected_dest_idx = self
                    .selected_destination
                    .and_then(|sd| destinations.iter().position(|(e, _)| *e == sd));
                sections.push(Box::new(
                    container!(
                        Label::new("Destination:").font(font, app),
                        Dropdown::new(
                            "",
                            vec2(WIDTH * 0.5, 40.0),
                            destinations
                                .iter()
                                .map(|(entity, name)| {
                                    (name.clone(), ManeuverMessages::SelectDestination(*entity))
                                })
                                .collect(),
                        )
                        .selected(selected_dest_idx)
                        .use_style(&STYLE),
                    )
                    .flow(Flow::Horizontal)
                    .cross_align(Align::Center)
                    .padding(vec2(12.0, 0.0)),
                ));
            }
        }

        // Result section
        let result_dv = self
            .computed_plan
            .as_ref()
            .map(|plan| format!("{:.0} m/s", plan.total_dv()))
            .unwrap_or(String::from(""));
        let can_afford_plan = self
            .computed_plan
            .as_ref()
            .map(|plan| plan.can_afford(craft_dv))
            .unwrap_or(false);
        let result_arrival_et = self
            .computed_plan
            .as_ref()
            .map(|plan| plan.arrival_et().as_calendar())
            .unwrap_or(String::from(""));

        sections.push(Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)));
        sections.push(Box::new(Label::new("RESULT").font(font_small_bold, app)));
        sections.push(Box::new(
            container![
                Label::new("dv: ").font(font, app).color(STYLE.text_primary),
                Label::new(result_dv)
                    .font(font, app)
                    .color(if can_afford_plan {
                        STYLE.positive
                    } else {
                        STYLE.negative
                    }),
            ]
            .padding(vec2(0.0, 0.0))
            .flow(Flow::Horizontal),
        ));
        sections.push(Box::new(
            Label::new(format!("Arrival: {result_arrival_et}")).font(font, app),
        ));

        // Footer buttons
        sections.push(Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)));
        let can_confirm = self
            .computed_plan
            .as_ref()
            .map(|p| p.can_afford(craft_dv))
            .unwrap_or(false);

        sections.push(Box::new(
            container![
                TextButton::new(Rectangle::new(0.0, 0.0, 100.0, 30.0), "Close")
                    .use_style(&STYLE)
                    .on_click(ManeuverMessages::Close),
                TextButton::new(Rectangle::new(0.0, 0.0, 100.0, 30.0), "Confirm")
                    .use_style_accented(&STYLE)
                    .on_click(ManeuverMessages::Confirm)
                    .active(can_confirm),
            ]
            .fixed_width(vec2(WIDTH, 0.0))
            .flow(Flow::Horizontal)
            .cross_align(Align::Center),
        ));

        self.modal = Modal::new(Box::new(
            Container::new(sections)
                .flow(Flow::Vertical)
                .padding(vec2(12.0, 12.0))
                .background_color(STYLE.bg_primary)
                .border(STYLE.border_primary, 1.0),
        ))
        .shown(true);
    }

    fn get_destinations(&self, craft: Entity, world: &World) -> Vec<(Entity, String)> {
        let parent = world
            .get::<&Parent>(craft)
            .expect("craft should have parent")
            .id;
        let mut binding = world.query::<(&State, &Body, &SceneObject, &Parent)>();
        binding
            .iter()
            .filter(|(_, (_, _, _, p))| p.id == parent)
            .map(|(entity, (_state, _body, scene_obj, _parent))| (entity, scene_obj.name.clone()))
            .collect()
    }

    fn compute_plan(&self, craft: Entity, world: &World) -> Option<ManeuverResult> {
        let kind = self.selected_kind.as_ref()?;
        let manuever_et = self.selected_date;

        let init_state = world.get::<&State>(craft);
        let parent = world
            .get::<&Parent>(craft)
            .expect("craft must have parent")
            .id;
        let parent_body = world.get::<&Body>(parent).expect("parent must be body");

        match kind {
            ManeuverKind::Transfer => {
                let to = self.selected_destination?;
                let target_state = world.get::<&State>(to).unwrap();
                let target_body = world.get::<&Body>(to).unwrap();
                let plan = plan_transfer(
                    &init_state.unwrap(),
                    &target_state,
                    target_body.body_radius,
                    manuever_et,
                    parent_body.mass(),
                    target_body.mass(),
                    TransferObjective::MinFuel,
                )
                .ok()?;
                Some(ManeuverResult::Transfer { to, plan })
            }
            ManeuverKind::Flyby => {
                let to = self.selected_destination?;
                let target_state = world.get::<&State>(to).unwrap();
                let target_body = world.get::<&Body>(to).unwrap();

                let plan = plan_flyby(
                    &init_state.unwrap(),
                    &target_state,
                    target_body.body_radius,
                    manuever_et,
                    parent_body.mass(),
                    target_body.mass(),
                    TransferObjective::MinFuel,
                )
                .ok()?;
                Some(ManeuverResult::Flyby { to, plan })
            }
            ManeuverKind::Escape => {
                let parent_state = world.get::<&State>(parent).unwrap();
                let grandparent = world.get::<&Parent>(parent).ok()?; // Could be None if orbiting the Sun
                let grandparent_body = world.get::<&Body>(grandparent.id).unwrap();

                let plan = plan_escape(
                    &init_state.unwrap(),
                    &parent_state,
                    manuever_et,
                    grandparent_body.mass(),
                    parent_body.mass(),
                );
                Some(ManeuverResult::Escape {
                    to: grandparent.id,
                    plan: plan.ok()?,
                })
            }
            ManeuverKind::Land => {
                let target_body = world.get::<&Body>(parent).unwrap();
                let plan = plan_landing(
                    &init_state.unwrap(),
                    target_body.body_radius,
                    manuever_et,
                    target_body.mu,
                )
                .ok()?;
                Some(ManeuverResult::Land { plan })
            }
            ManeuverKind::Launch => {
                let landed = world.get::<&Landed>(craft).ok()?;
                let parent_state = world.get::<&State>(parent).unwrap();
                let grandparent = world.get::<&Parent>(parent).unwrap();
                let grandparent_body = world.get::<&Body>(grandparent.id).unwrap();

                let plan = plan_launch(
                    landed.offset,
                    &parent_state,
                    parent_body.body_radius,
                    manuever_et,
                    grandparent_body.mass(),
                    parent_body.mass(),
                )
                .ok()?;
                Some(ManeuverResult::Launch { plan })
            }
        }
    }
}
