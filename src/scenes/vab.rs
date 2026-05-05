use std::collections::HashMap;

use crate::{
    components::{
        inventory::PartInventory,
        parts::{PartDef, PartRegistry},
    },
    ui::{container::Container, style::STYLE},
};
use apricot::{app::App, font::FontId, rectangle::Rectangle};

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

pub struct VabUi {
    modal: Modal<VabMessages>,

    pub stages: Vec<PartDef>,
    pub payload: Option<PartDef>,

    available_parts: HashMap<String, u32>,
    registry: PartRegistry,
}

#[derive(Clone, Debug)]
enum VabMessages {
    AddToStack(String),
    RemoveFromStack,
    SetPayload(String),
    UnsetPayload,
    Build,
    Close,
}

impl VabUi {
    pub fn new() -> Self {
        Self {
            modal: Modal::new(Box::new(container![])),
            stages: Vec::new(),
            payload: None,
            available_parts: HashMap::new(),
            registry: PartRegistry::new(),
        }
    }

    pub fn update(&mut self, app: &App) -> bool {
        for msg in recv_msgs(app, &mut self.modal) {
            println!("{msg:?}");
            match msg {
                VabMessages::AddToStack(stage_id) => {
                    let stage = self.registry.get(&stage_id).unwrap();
                    self.stages.push(stage.clone());
                    let count = self.available_parts.get(&stage_id).unwrap();
                    self.available_parts.insert(stage_id, count - 1);
                    self.rebuild_modal(app);
                }
                VabMessages::SetPayload(payload_id) => {
                    let payload = self.registry.get(&payload_id).unwrap();
                    self.payload = Some(payload.clone());
                    let count = self.available_parts.get(&payload_id).unwrap();
                    self.available_parts.insert(payload_id, count - 1);
                    self.rebuild_modal(app);
                }
                VabMessages::RemoveFromStack => {
                    let stage = self.stages.pop().unwrap();
                    let count = self.available_parts.get(&stage.id).unwrap();
                    self.available_parts.insert(stage.id, count + 1);
                    self.rebuild_modal(app);
                }
                VabMessages::UnsetPayload => {
                    let old_id = self.payload.clone().unwrap().id;
                    self.payload = None;
                    self.available_parts.insert(old_id, 1);
                    self.rebuild_modal(app);
                }
                VabMessages::Build => {
                    self.modal.set_shown(false);
                    return true;
                }
                VabMessages::Close => {
                    self.modal.set_shown(false);
                }
            }
        }

        false
    }

    pub fn render(&self, app: &App) {
        self.modal.render(app);
    }

    pub fn show(&mut self, inventory: &PartInventory, registry: &PartRegistry, app: &App) {
        self.available_parts = inventory.parts.clone();
        self.stages = vec![];
        self.payload = None;
        self.registry = registry.clone();

        self.rebuild_modal(app);
    }

    fn rebuild_modal(&mut self, app: &App) {
        let font = app.renderer.get_font_id_from_name("font").unwrap();

        self.modal = Modal::new(Box::new(
            container![
                // Title
                Label::new("Vehicle Assembly Building:").font(font, app),
                // Middle section
                container![
                    Container::new(self.build_rocket(font, app))
                        .flow(Flow::Vertical)
                        .cross_align(Align::Center),
                    Container::new(self.build_available_parts(font, app))
                        .flow(Flow::Vertical)
                        .cross_align(Align::Center),
                ]
                .flow(Flow::Horizontal)
                .cross_align(Align::Center),
                // Bottom row
                container![
                    TextButton::new(Rectangle::new(100.0, 120.0, 200.0, 30.0,), "Close!",)
                        .background_color(STYLE.bg_primary)
                        .hovered_color(STYLE.bg_hover)
                        .border(STYLE.border_primary, 1.0)
                        .on_click(VabMessages::Close),
                    TextButton::new(Rectangle::new(100.0, 120.0, 200.0, 30.0,), "Build!",)
                        .background_color(STYLE.bg_primary)
                        .hovered_color(STYLE.bg_hover)
                        .border(STYLE.border_primary, 1.0)
                        .on_click(VabMessages::Build)
                ]
                .flow(Flow::Horizontal)
                .cross_align(Align::Center),
            ]
            .cross_align(Align::Center)
            .background_color(STYLE.bg_primary),
        ))
        .shown(true);
    }

    fn build_rocket(&self, font: FontId, app: &App) -> Vec<Box<dyn Widget<VabMessages>>> {
        let mut widgets = vec![];

        let payload: Vec<Box<dyn Widget<VabMessages>>> = self
            .payload
            .iter()
            .map(|payload| {
                Box::new(container![
                    Label::new(payload.name.clone()).font(font, app),
                    TextButton::new(Rectangle::new(100.0, 120.0, 200.0, 30.0,), "Remove",)
                        .background_color(STYLE.bg_primary)
                        .hovered_color(STYLE.bg_hover)
                        .border(STYLE.border_primary, 1.0)
                        .on_click(VabMessages::UnsetPayload)
                ]) as Box<dyn Widget<VabMessages>>
            })
            .collect();

        let mut stages: Vec<Box<dyn Widget<VabMessages>>> = self
            .stages
            .iter()
            .map(|stage| {
                Box::new(container![Label::new(stage.name.clone()).font(font, app),])
                    as Box<dyn Widget<VabMessages>>
            })
            .collect();
        if !self.stages.is_empty() {
            stages.push(Box::new(
                TextButton::new(Rectangle::new(100.0, 120.0, 200.0, 30.0), "Remove")
                    .background_color(STYLE.bg_primary)
                    .hovered_color(STYLE.bg_hover)
                    .border(STYLE.border_primary, 1.0)
                    .on_click(VabMessages::RemoveFromStack),
            ))
        }

        widgets.extend(payload);
        widgets.extend(stages);

        widgets
    }

    fn build_available_parts(&self, font: FontId, app: &App) -> Vec<Box<dyn Widget<VabMessages>>> {
        let mut widgets = vec![];

        let payloads: Vec<Box<dyn Widget<VabMessages>>> = self
            .available_parts
            .iter()
            .filter_map(|(part_id, count)| {
                let part = self.registry.get(part_id).unwrap();
                if part.fuel.is_some() {
                    return None;
                }
                if *count == 0 {
                    return None;
                }

                Some(Box::new(container![
                    Label::new(format!("{} ({count})", part.name.clone())).font(font, app),
                    TextButton::new(Rectangle::new(100.0, 120.0, 200.0, 30.0,), "Set as payload",)
                        .background_color(STYLE.bg_primary)
                        .hovered_color(STYLE.bg_hover)
                        .border(STYLE.border_primary, 1.0)
                        .on_click(VabMessages::SetPayload(part_id.clone()))
                ]) as Box<dyn Widget<VabMessages>>)
            })
            .collect();

        let stages: Vec<Box<dyn Widget<VabMessages>>> = self
            .available_parts
            .iter()
            .filter_map(|(part_id, count)| {
                let part = self.registry.get(part_id).unwrap();
                let _ = part.fuel?;
                if *count == 0 {
                    return None;
                }
                Some(Box::new(container![
                    Label::new(format!("{} ({count})", part.name.clone())).font(font, app),
                    TextButton::new(Rectangle::new(100.0, 120.0, 200.0, 30.0,), "Add to stack",)
                        .background_color(STYLE.bg_primary)
                        .hovered_color(STYLE.bg_hover)
                        .border(STYLE.border_primary, 1.0)
                        .on_click(VabMessages::AddToStack(part_id.clone()))
                ]) as Box<dyn Widget<VabMessages>>)
            })
            .collect();

        widgets.extend(payloads);
        widgets.extend(stages);

        widgets
    }
}
