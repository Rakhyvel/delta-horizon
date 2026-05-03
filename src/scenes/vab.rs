use std::collections::HashMap;

use crate::{
    components::{inventory::PartInventory, parts::PartRegistry},
    ui::container::Container,
};
use apricot::{app::App, rectangle::Rectangle};
use nalgebra_glm::vec4;

use crate::{
    components::craft::{Payload, Stage},
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

    stages: Vec<Stage>,
    payload: Option<Payload>,

    available_parts: HashMap<String, u32>,
}

#[derive(Clone)]
enum VabMessages {
    Close,
}

impl VabUi {
    pub fn new() -> Self {
        Self {
            modal: Modal::new(Box::new(container![])),
            stages: Vec::new(),
            payload: None,
            available_parts: HashMap::new(),
        }
    }

    pub fn show(&mut self, inventory: &PartInventory, registry: &PartRegistry, app: &App) {
        self.available_parts = inventory.parts.clone();

        self.rebuild_modal(registry, app);

        self.modal.set_shown(true);
    }

    pub fn update(&mut self, app: &App) {
        for msg in recv_msgs(app, &mut self.modal) {
            match msg {
                VabMessages::Close => {
                    self.modal.set_shown(false);
                }
            }
        }
    }

    pub fn render(&self, app: &App) {
        self.modal.render(app);
    }

    fn rebuild_modal(&mut self, _registry: &PartRegistry, app: &App) {
        let font = app.renderer.get_current_font().unwrap();

        self.modal = Modal::new(Box::new(
            container![
                // Title
                Label::new("Vehicle Assembly Building:", &font),
                // Middle section
                container![
                    container![
                        Label::new("Rocket so far:", &font),
                        Label::new("Available payloads and stages:", &font),
                    ]
                    .flow(Flow::Vertical)
                    .cross_align(Align::Center),
                    container![
                        Label::new("Rocket so far:", &font),
                        Label::new("Available payloads and stages:", &font),
                    ]
                    .flow(Flow::Vertical)
                    .cross_align(Align::Center),
                ]
                .flow(Flow::Horizontal)
                .cross_align(Align::Center),
                // Bottom row
                container![
                    TextButton::new(
                        Rectangle::new(100.0, 120.0, 200.0, 30.0,),
                        "Close!",
                        vec4(0.02, 0.07, 0.11, 1.0),
                        vec4(1.0, 1.0, 1.0, 0.5),
                    )
                    .on_click(VabMessages::Close),
                    TextButton::new(
                        Rectangle::new(100.0, 120.0, 200.0, 30.0,),
                        "Build!",
                        vec4(0.02, 0.07, 0.11, 1.0),
                        vec4(1.0, 1.0, 1.0, 0.5),
                    )
                    .on_click(VabMessages::Close)
                ]
                .flow(Flow::Horizontal)
                .cross_align(Align::Center),
            ]
            .cross_align(Align::Center)
            .background(vec4(91.25, 160.0, 228.75, 51.0) / 255.0),
        ));
    }
}
