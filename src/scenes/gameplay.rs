//! This module is responsible for defining the gameplay scene.

use std::{collections::HashMap, f64::consts::PI};

use apricot::{
    app::{App, Scene},
    bvh::BVH,
    camera::{Camera, ProjectionKind},
    font::Font,
    high_precision::{self, WorldPosition},
    opengl::create_program,
    rectangle::Rectangle,
    render_core::{LinePathComponent, ModelComponent},
    shadow_map::DirectionalLightSource,
};
use hecs::{Entity, World};
use nalgebra_glm::{vec2, vec3, vec4, DMat4, DVec3, IVec2, Mat4, Vec2};
use sdl2::keyboard::Scancode;

use crate::{
    astro::{
        epoch::EphemerisTime,
        state::State,
        transfer::{plan_transfer, sphere_of_influence, TransferObjective},
    },
    components::craft::{replace_line_path, AssociatedEntity, Command},
    container,
    generation::{lexicon::Lexicon, solar_system_gen::SUN_MU},
    scenes::events::{Event, EventQueue},
    ui::{label::Label, text_button::TextButton},
};

use crate::{
    components::{
        body::{spawn_body, Body, Category, Parent, SceneObject},
        craft::{spawn_craft, spawn_landed_craft, Craft, Landed},
    },
    generation::solar_system_gen::{self},
    ui::{
        container::Container,
        texture_button::TextureButton,
        widget::{recv_msgs, Widget},
    },
};

/// Object file data, used for meshes
pub const QUAD_XY_DATA: &[u8] = include_bytes!("../../res/quad-xy.obj");
pub const ICO_DATA: &[u8] = include_bytes!("../../res/ico-sphere.obj");
pub const UV_DATA: &[u8] = include_bytes!("../../res/uv-sphere.obj");
pub const CONE_DATA: &[u8] = include_bytes!("../../res/cone.obj");

/// Struct that contains info about the game state
pub struct Gameplay {
    /// The world where all the entities live
    world: World,
    /// The camera used for rendering 3d models
    camera_3d: high_precision::Camera,
    /// The sun's light source
    directional_light: DirectionalLightSource,
    /// A bounding-volume hierarchy, a container that stores models and allows for efficient lookup for fast rendering
    bvh: BVH<Entity>,

    selection: SelectionState,
    hovered: Option<Entity>,

    /// Up-down view angle
    phi: f64,
    /// Side-side view angle
    theta: f64,
    /// How far the camera swivels around the currently selected body
    distance: f64,

    /// Used for tab key latch
    prev_tab_state: bool,

    gui: Container<Message>,

    // Events and timeline
    event_queue: EventQueue,
    current_et: EphemerisTime,
    animation_start_et: EphemerisTime,
    animation_target_et: EphemerisTime,
    animation_start_real: f64,
}

#[derive(Clone)]
enum Message {
    NextTurn,
    CraftCommand(Command),
}

#[derive(Debug)]
enum SelectionKind {
    Craft,
    Body,
}

struct SelectionState {
    pub crafts: Vec<Entity>,
    pub bodies: Vec<Entity>,
    pub selected: Option<usize>,
    pub kind: SelectionKind,

    // For swoosh animation
    pub selected_pos: DVec3,
    pub prev_selected_pos: DVec3,
    pub transition: f64,
}

impl SelectionState {
    pub fn new(crafts: Vec<Entity>, bodies: Vec<Entity>) -> Self {
        Self {
            crafts,
            bodies,
            selected: None,
            kind: SelectionKind::Craft,
            selected_pos: vec3(0.0, 0.0, 0.0),
            prev_selected_pos: vec3(0.0, 0.0, 0.0),
            transition: 0.0,
        }
    }

    pub fn selected_entity(&self) -> Option<Entity> {
        self.selected.map(|s| self.curr_sel_track()[s])
    }

    pub fn set_selected(&mut self, entity: Entity, app_seconds: f64) {
        let found = self
            .crafts
            .iter()
            .position(|e| *e == entity)
            .map(|x| (x, SelectionKind::Craft))
            .or(self
                .bodies
                .iter()
                .position(|e| *e == entity)
                .map(|x| (x, SelectionKind::Body)));

        if let Some((idx, kind)) = found {
            self.selected = Some(idx);
            self.kind = kind;

            self.prev_selected_pos = self.selected_pos;
            self.transition = app_seconds;
        }
    }

    pub fn prev(&mut self, app_seconds: f64) {
        if let Some(selected) = self.selected {
            let mut new_selection = selected;
            if selected == 0 {
                new_selection = self.curr_sel_track().len() - 1;
            } else {
                new_selection -= 1;
            }
            self.selected = Some(new_selection);
        } else {
            self.selected = Some(0);
        }

        self.prev_selected_pos = self.selected_pos;
        self.transition = app_seconds;
    }

    pub fn next(&mut self, app_seconds: f64) {
        if let Some(selected) = self.selected {
            let mut new_selection = selected + 1;
            if new_selection >= self.curr_sel_track().len() {
                new_selection = 0;
            }
            self.selected = Some(new_selection);
        } else {
            self.selected = Some(0);
        }

        self.prev_selected_pos = self.selected_pos;
        self.transition = app_seconds;
    }

    pub fn is_animating(&self, app_seconds: f64) -> bool {
        app_seconds - self.transition < 1.0
    }

    fn curr_sel_track(&self) -> &Vec<Entity> {
        match self.kind {
            SelectionKind::Body => &self.bodies,
            SelectionKind::Craft => &self.crafts,
        }
    }
}

impl Scene for Gameplay {
    /// Update the scene every tick
    fn update(&mut self, app: &App) {
        // Handle all the messages from UI
        for msg in recv_msgs(app, &mut self.gui) {
            match msg {
                Message::NextTurn => {
                    if !self.is_animating() {
                        self.plan_commands();
                        if let Some((&next_event_time, _)) = self.event_queue.events.iter().next() {
                            self.animation_start_et = self.current_et;
                            self.animation_target_et = next_event_time;
                            self.animation_start_real = app.seconds as f64;
                        }
                    }
                }
                Message::CraftCommand(cmd) => {
                    if let Some(selected) = self.selection.selected_entity() {
                        self.world.get::<&mut Craft>(selected).unwrap().command = Some(cmd);
                    }
                }
            }
        }

        if self.is_animating() {
            const TURN_TIME: f64 = 1.5;
            let t = ((app.seconds as f64 - self.animation_start_real) / TURN_TIME).min(1.0);
            let eased = t;

            // Interpolate ET between start and target
            self.current_et = self
                .animation_start_et
                .lerp(self.animation_target_et, eased);

            // Animation finished
            if t >= 1.0 {
                self.current_et = self.animation_target_et;
                let due = self.event_queue.pop_due(self.current_et);
                for event in due {
                    self.handle_event(event, app);
                }
                self.gui = self.rebuild_gui(app);
            }
        }

        self.control(app);
        self.orbit_system(app);
        self.landed_system(app);
        self.mouse_hover_system(app);
        self.select_system();
        self.line_path_system(app);
        self.camera_update(app);

        // Delete anything we want deleted
        app.renderer.flush_deletion_queue();
    }

    /// Render the scene to the screen when time allows
    fn render(&mut self, app: &App) {
        // Set everything up
        self.directional_light.light_dir = -self.camera_3d.inner.position().normalize();
        app.renderer.set_camera(self.camera_3d.inner);
        let font = app.renderer.get_font_id_from_name("font").unwrap();
        app.renderer.set_font(font);

        // Draw the 3D stuff
        app.renderer.set_color(vec4(0.01, 0.01, 0.01, 1.0));
        app.renderer.clear();
        self.render_dots(app);
        app.renderer.directional_light_system(
            &mut self.directional_light,
            &mut self.world,
            &self.bvh,
        );
        app.renderer.render_3d_models_system(
            &mut self.world,
            &self.directional_light,
            &self.bvh,
            Some(&self.camera_3d),
            false,
        );
        app.renderer
            .render_3d_line_paths(&self.world, Some(&self.camera_3d));

        // Draw the 2D stuff
        // Draw selected reticle
        if !self.selection.is_animating(app.seconds as f64) {
            let reticle_texture = app.renderer.get_texture_id_from_name("reticle").unwrap();
            const WIDTH: f32 = 16.0;
            app.renderer.copy_texture(
                Rectangle::new(
                    (app.window_size.x as f32 - WIDTH) * 0.5,
                    (app.window_size.y as f32 - WIDTH) * 0.5,
                    WIDTH,
                    WIDTH,
                ),
                reticle_texture,
                Rectangle::new(0.0, 0.0, WIDTH, WIDTH),
            );
        }

        // Draw hovered reticle
        if let (Some(hovered), Some(selected)) = (self.hovered, self.selection.selected_entity()) {
            if hovered != selected {
                let hovered_world_pos = self.world.get::<&WorldPosition>(hovered).unwrap().pos;
                let scene_obj = self.world.get::<&SceneObject>(hovered).unwrap();

                let relative_pos = hovered_world_pos - self.camera_3d.world_pos;
                let screen_pos = self
                    .world_to_screen(relative_pos, app)
                    .expect("we're hovering over it so it should exist");

                let width = 16.0;

                let reticle_texture = app.renderer.get_texture_id_from_name("reticle").unwrap();
                app.renderer.copy_texture(
                    Rectangle::new(
                        screen_pos.x - width * 0.5,
                        screen_pos.y - width * 0.5,
                        width,
                        width,
                    ),
                    reticle_texture,
                    Rectangle::new(0.0, 0.0, 16.0, 16.0),
                );
                app.renderer
                    .draw_text(screen_pos + vec2(8.0, 8.0), &scene_obj.name);
            }
        }

        // Draw GUI
        self.gui.render(app);
    }
}

impl Gameplay {
    /// Constructs a new Gameplay struct with everything setup
    /// TODO: Most of this stuff will need to be moved to the init scene. Remind me to make an issue for this!
    pub fn new(app: &App) -> Self {
        let mut world = World::new();

        // Add programs to the renderer
        app.renderer.add_program(
            create_program(
                include_str!("../shaders/3d.vert"),
                include_str!("../shaders/3d.frag"),
            )
            .unwrap(),
            Some("3d"),
        );
        app.renderer.add_program(
            create_program(
                include_str!("../shaders/2d.vert"),
                include_str!("../shaders/2d.frag"),
            )
            .unwrap(),
            Some("2d"),
        );
        app.renderer.add_program(
            create_program(
                include_str!("../shaders/shadow.vert"),
                include_str!("../shaders/shadow.frag"),
            )
            .unwrap(),
            Some("shadow"),
        );
        app.renderer.add_program(
            create_program(
                include_str!("../shaders/2d.vert"),
                include_str!("../shaders/solid-color.frag"),
            )
            .unwrap(),
            Some("2d-solid"),
        );
        app.renderer.add_program(
            create_program(
                include_str!("../shaders/3d.vert"),
                include_str!("../shaders/solid-color.frag"),
            )
            .unwrap(),
            Some("3d-solid"),
        );
        app.renderer.add_program(
            create_program(
                include_str!("../shaders/line.vert"),
                include_str!("../shaders/line.frag"),
            )
            .unwrap(),
            Some("line"),
        );

        // Setup the mesh manager
        app.renderer
            .add_mesh_from_obj(QUAD_XY_DATA, Some("quad-xy"));
        app.renderer.add_mesh_from_obj(UV_DATA, Some("uv"));
        app.renderer.add_mesh_from_obj(ICO_DATA, Some("ico"));
        app.renderer.add_mesh_from_obj(CONE_DATA, Some("cone"));

        // Setup the texture manager
        app.renderer
            .add_texture_from_png("res/sun.png", Some("sun"));
        app.renderer
            .add_texture_from_png("res/venus.png", Some("venus"));
        app.renderer
            .add_texture_from_png("res/earth.png", Some("earth"));
        app.renderer
            .add_texture_from_png("res/moon.png", Some("moon"));
        app.renderer
            .add_texture_from_png("res/jupiter.png", Some("jupiter"));
        app.renderer
            .add_texture_from_png("res/europa.png", Some("europa"));
        app.renderer
            .add_texture_from_png("res/uranus.png", Some("uranus"));
        app.renderer
            .add_texture_from_png("res/next-turn.png", Some("next-turn"));
        app.renderer
            .add_texture_from_png("res/next-turn-hover.png", Some("next-turn-hover"));
        app.renderer
            .add_texture_from_png("res/reticle.png", Some("reticle"));

        // Setup the font manager
        app.renderer
            .add_font("res/Consolas.ttf", "font", 16, sdl2::ttf::FontStyle::NORMAL);

        let mut bvh = BVH::<Entity>::new();

        let sun_entity = spawn_body(
            Body {
                category: Category::Star,
                body_radius: 110.0,
                rotation_period_hours: 0.0,
                rotation: 0.0,
                atmos_pressure: 1000000.0,
                temperature: 5778.0,
                core_mass_fraction: 0.0,
                magnetic_field: true,
                density: 1.0,
                mu: SUN_MU,
            },
            State::circular(0.1, EphemerisTime::new(rand::random()), 1.0),
            SceneObject {
                bvh_node_id: None,
                name: String::from("The Sun"),
            },
            None,
            &mut world,
            &app.renderer,
            &mut bvh,
        );

        let mut bodies = vec![sun_entity];
        let mut crafts = vec![];

        let (_lexicon, _node_count) = Lexicon::create("res/names.txt", "res/names.lex");
        let lexicon = Lexicon::read("res/names.lex");

        let mut habitable_planet = 0;
        let mut habitable_planet_mu = 0.0;
        let mut habitable_planet_radius = 0.0;
        let planets = solar_system_gen::generate();
        for system in planets {
            let planet_habitable = system.planet.0.habitable();
            let name = lexicon.generate_word(7);
            println!("Planet: {}", name);
            let planet_entity = spawn_body(
                system.planet.0,
                system.planet.1,
                SceneObject {
                    bvh_node_id: None,
                    name,
                },
                Some(Parent { id: sun_entity }),
                &mut world,
                &app.renderer,
                &mut bvh,
            );
            if !system.moons.is_empty() {
                habitable_planet = bodies.len();
                habitable_planet_mu = system.planet.0.mu;
                habitable_planet_radius = system.planet.0.body_radius;
            }

            bodies.push(planet_entity);

            for moon in system.moons {
                let name = lexicon.generate_word(10);
                println!("Moon: {}", name);
                let moon_entity = spawn_body(
                    moon.0,
                    moon.1,
                    SceneObject {
                        bvh_node_id: None,
                        name,
                    },
                    Some(Parent { id: planet_entity }),
                    &mut world,
                    &app.renderer,
                    &mut bvh,
                );
                bodies.push(moon_entity);
            }
        }

        let state = State::from_kepler(
            habitable_planet_radius * 10.0,
            0.3,
            PI * 90.6 / 180.0,
            0.0,
            PI / 3.0,
            0.0,
            EphemerisTime::new(0),
            habitable_planet_mu,
        );
        let craft_entity = spawn_craft(
            state,
            SceneObject {
                bvh_node_id: None,
                name: String::from("craft"),
            },
            Some(Parent {
                id: bodies[habitable_planet],
            }),
            &mut world,
            &app.renderer,
            &mut bvh,
        );
        crafts.push(craft_entity);
        let landed_craft_entity = spawn_landed_craft(
            SceneObject {
                bvh_node_id: None,
                name: String::from("landed craft"),
            },
            Some(Parent {
                id: bodies[habitable_planet],
            }),
            &mut world,
            &app.renderer,
            &mut bvh,
        );
        crafts.push(landed_craft_entity);

        let gui = container![
            TextureButton::new(
                Rectangle::new(
                    app.window_size.x as f32 - 100.0,
                    app.window_size.y as f32 - 120.0,
                    90.0,
                    90.0,
                ),
                app.renderer.get_texture_id_from_name("next-turn").unwrap(),
                app.renderer
                    .get_texture_id_from_name("next-turn-hover")
                    .unwrap(),
            )
            .on_click(Message::NextTurn),
            TextButton::new(
                Rectangle::new(100.0, 120.0, 200.0, 30.0,),
                "Click me!",
                vec4(0.02, 0.07, 0.11, 1.0),
                vec4(1.0, 1.0, 1.0, 0.5),
            )
            .on_click(Message::NextTurn),
        ]
        .at(vec2(100.0, 100.0));

        Self {
            world,
            camera_3d: high_precision::Camera {
                world_pos: vec3(1.0, 1.0, 1.0),
                inner: Camera::new(
                    vec3(1.0, 0.0, 1.0),
                    vec3(0.0, 0.0, 0.0),
                    vec3(0.0, 0.0, 1.0),
                    ProjectionKind::Perspective {
                        fov: 0.65,
                        far: 10000000.0,
                    },
                ),
            },
            bvh,
            directional_light: DirectionalLightSource::new(
                Camera::new(
                    vec3(0.0, 0.0, 0.0),
                    vec3(0.0, 10.0, 0.0),
                    vec3(0.0, 0.0, 1.0),
                    ProjectionKind::Orthographic {
                        // These do not matter for now, they're reset later
                        left: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        top: 0.0,
                        near: 0.0,
                        far: 0.0,
                    },
                ),
                vec3(-1.0, 0.0, 0.0),
                1024,
            ),

            selection: SelectionState::new(crafts, bodies),
            hovered: None,

            phi: 2.5,
            theta: -PI / 4.0,
            distance: 20.0,
            prev_tab_state: false,

            gui,

            current_et: EphemerisTime::from_years(-200.0),
            animation_start_et: EphemerisTime::from_years(-200.0),
            animation_target_et: EphemerisTime::from_years(-200.0),
            animation_start_real: 0.0,
            event_queue: EventQueue::new(),
        }
    }

    fn is_animating(&self) -> bool {
        self.current_et < self.animation_target_et
    }

    /// Changes various game state based on user mouse and keyboard input
    fn control(&mut self, app: &App) {
        let curr_tab_state = app.keys[Scancode::Tab as usize];
        let curr_shift_state =
            app.keys[Scancode::LShift as usize] || app.keys[Scancode::RShift as usize];
        if curr_tab_state && !self.prev_tab_state {
            if curr_shift_state {
                self.selection.prev(app.seconds as f64);
            } else {
                self.selection.next(app.seconds as f64);
            }
            self.gui = self.rebuild_gui(app);
        }
        self.prev_tab_state = curr_tab_state;

        const MIN_DISTANCE: f64 = 0.12;
        const MAX_DISTANCE: f64 = 1e6;

        let control_speed = 0.005;
        let zoom_control_speed = 0.15 * (self.distance - MIN_DISTANCE);
        if app.mouse_left_down {
            self.phi -= control_speed * (app.mouse_vel.x as f64);
            self.theta = (self.theta - control_speed * (app.mouse_vel.y as f64))
                .max(control_speed - PI / 2.0)
                .min(PI / 2.0 - control_speed);
        }

        self.distance = (self.distance - zoom_control_speed * (app.mouse_wheel as f64))
            .clamp(0.0, MAX_DISTANCE);
    }

    fn rebuild_gui(&self, app: &App) -> Container<Message> {
        let font = app.renderer.get_current_font().unwrap();

        let mut widgets: Vec<Box<dyn Widget<Message>>> = vec![];
        widgets.extend(self.build_footer_widgets(app, &font));

        let selected = self.selection.selected_entity();
        if let Some(selected) = selected {
            widgets.extend(self.build_selection_widgets(selected, &font));
        }

        Container::new(widgets)
    }

    fn build_footer_widgets(&self, app: &App, font: &Font) -> Vec<Box<dyn Widget<Message>>> {
        vec![
            Box::new(
                TextureButton::new(
                    Rectangle::new(
                        app.window_size.x as f32 - 100.0,
                        app.window_size.y as f32 - 120.0,
                        90.0,
                        90.0,
                    ),
                    app.renderer.get_texture_id_from_name("next-turn").unwrap(),
                    app.renderer
                        .get_texture_id_from_name("next-turn-hover")
                        .unwrap(),
                )
                .on_click(Message::NextTurn),
            ),
            Box::new(Label::new(format!("ET: {:?} us", self.current_et), font)),
        ]
    }

    fn build_selection_widgets(
        &self,
        selected: Entity,
        font: &Font,
    ) -> Vec<Box<dyn Widget<Message>>> {
        let scene_object = self.world.get::<&SceneObject>(selected).unwrap();
        let mut widgets: Vec<Box<dyn Widget<Message>>> =
            vec![Box::new(Label::new(scene_object.name.clone(), font))];

        if self.world.get::<&Craft>(selected).is_ok() {
            if let Some(status) = self.build_orbit_widgets(selected, font) {
                widgets.extend(status);
            }
            if let Some(status) = self.build_landed_widgets(selected, font) {
                widgets.extend(status);
            }
        }

        widgets
    }

    fn build_orbit_widgets(
        &self,
        selected: Entity,
        font: &Font,
    ) -> Option<Vec<Box<dyn Widget<Message>>>> {
        let _orbit = self.world.get::<&State>(selected).ok()?;
        let parent = self.world.get::<&Parent>(selected).ok()?;
        let parent_scene_object = self.world.get::<&SceneObject>(parent.id).unwrap();

        let mut widgets: Vec<Box<dyn Widget<Message>>> = vec![Box::new(Label::new(
            format!("Status: Orbiting {}", parent_scene_object.name),
            font,
        ))];

        // Land on body
        widgets.push(Box::new(
            TextButton::<Message>::new(
                Rectangle::new(100.0, 120.0, 220.0, 40.0),
                format!("Land on {}", parent_scene_object.name),
                vec4(0.02, 0.07, 0.11, 1.0),
                vec4(1.0, 1.0, 1.0, 0.5),
            )
            .on_click(Message::CraftCommand(Command::Land {})),
        ));

        // Escape transfer to grandparent body
        if let Ok(grandparent) = self.world.get::<&Parent>(parent.id) {
            let grandparent_scene_object = self.world.get::<&SceneObject>(grandparent.id).unwrap();
            widgets.push(Box::new(
                TextButton::<Message>::new(
                    Rectangle::new(100.0, 120.0, 220.0, 40.0),
                    format!("Transfer to {}", grandparent_scene_object.name),
                    vec4(0.02, 0.07, 0.11, 1.0),
                    vec4(1.0, 1.0, 1.0, 0.5),
                )
                .on_click(Message::CraftCommand(Command::Transfer {
                    to: grandparent.id,
                })),
            ));
        }

        // Transfers to sibling bodies
        let mut binding = self.world.query::<(&State, &Body, &SceneObject, &Parent)>();
        let transfers = binding
            .iter()
            .filter(|(_, (_, _, _, p))| p.id == parent.id)
            .map(|(entity, (_, _, scene_obj, _))| {
                Box::new(
                    TextButton::<Message>::new(
                        Rectangle::new(100.0, 120.0, 220.0, 40.0),
                        format!("Transfer to {}", scene_obj.name),
                        vec4(0.02, 0.07, 0.11, 1.0),
                        vec4(1.0, 1.0, 1.0, 0.5),
                    )
                    .on_click(Message::CraftCommand(Command::Transfer { to: entity })),
                ) as Box<dyn Widget<Message>>
            });

        widgets.extend(transfers);
        Some(widgets)
    }

    fn build_landed_widgets(
        &self,
        selected: Entity,
        font: &Font,
    ) -> Option<Vec<Box<dyn Widget<Message>>>> {
        let _landed = self.world.get::<&Landed>(selected).ok()?;
        let parent = self.world.get::<&Parent>(selected).ok()?;
        let parent_scene_object = self.world.get::<&SceneObject>(parent.id).unwrap();

        Some(vec![
            Box::new(Label::new(
                format!("Status: Landed on {}", parent_scene_object.name),
                font,
            )),
            Box::new(
                TextButton::<Message>::new(
                    Rectangle::new(100.0, 120.0, 220.0, 40.0),
                    "Orbit",
                    vec4(0.02, 0.07, 0.11, 1.0),
                    vec4(1.0, 1.0, 1.0, 0.5),
                )
                .on_click(Message::CraftCommand(Command::Orbit)),
            ),
        ])
    }

    fn plan_commands(&mut self) {
        let crafts_with_commands: Vec<(Entity, Command)> = self
            .world
            .query::<(&mut Craft,)>()
            .iter()
            .filter_map(|(entity, (craft,))| craft.command.take().map(|cmd| (entity, cmd)))
            .collect();

        for (entity, command) in crafts_with_commands {
            match command {
                Command::Transfer { to } => {
                    let init_state = self.world.get::<&State>(entity).unwrap();
                    let target_state = self.world.get::<&State>(to).unwrap();
                    let target_body = self.world.get::<&Body>(to).unwrap();
                    let parent = self.world.get::<&Parent>(entity).unwrap().id;
                    let parent_body = self.world.get::<&Body>(parent).unwrap();
                    let transfer = plan_transfer(
                        &init_state,
                        &target_state,
                        self.current_et,
                        parent_body.mass(),
                        target_body.mass(),
                        TransferObjective::MinFuel,
                    );

                    let departure_time = transfer.transfer_state.t;
                    let arrival_time = transfer.flyby_state.t;
                    let circ_time = transfer.circ_state.t;

                    self.event_queue.push(
                        departure_time,
                        Event::ManeuverReady {
                            craft: entity,
                            to,
                            transfer_orbit: transfer.transfer_state,
                            soi_radius: Some(transfer.soi_radius * 1.1),
                        },
                    );

                    self.event_queue.push(
                        arrival_time,
                        Event::SoiChange {
                            craft: entity,
                            new_parent: to,
                            flyby_orbit: transfer.flyby_state,
                            soi_radius: transfer.soi_radius * 1.6,
                        },
                    );

                    self.event_queue.push(
                        circ_time,
                        Event::ManeuverReady {
                            craft: entity,
                            to,
                            transfer_orbit: transfer.circ_state,
                            soi_radius: Some(transfer.soi_radius * 1.1),
                        },
                    );
                }
                Command::Orbit => {
                    self.event_queue
                        .push(self.animation_target_et, Event::TakeOff { craft: entity });
                }
                Command::Land => {
                    self.event_queue
                        .push(self.animation_target_et, Event::Land { craft: entity });
                }
                _ => {}
            }
        }
    }

    fn handle_event(&mut self, event: Event, app: &App) {
        match event {
            Event::SoiChange {
                craft,
                new_parent,
                flyby_orbit,
                soi_radius,
            } => {
                let parent_world_pos = self.world.get::<&WorldPosition>(new_parent).unwrap().pos;
                let parent_mu = { self.world.get::<&Body>(new_parent).unwrap().mu };
                replace_line_path(
                    &mut self.world,
                    &app.renderer,
                    craft,
                    Some((
                        WorldPosition {
                            pos: parent_world_pos,
                        },
                        Parent { id: new_parent },
                        LinePathComponent::new(flyby_orbit.generate_orbit_vertices(
                            8192,
                            parent_mu,
                            Some(soi_radius),
                        )),
                        AssociatedEntity { associate: craft },
                    )),
                );
                self.world.remove_one::<State>(craft).ok();
                self.world
                    .insert(craft, (flyby_orbit, Parent { id: new_parent }))
                    .unwrap();
            }
            Event::ManeuverReady {
                craft,
                to,
                transfer_orbit,
                soi_radius,
            } => {
                let parent = self.world.get::<&Parent>(craft).unwrap().id;
                let parent_world_pos = self.world.get::<&WorldPosition>(parent).unwrap().pos;
                let parent_mu = { self.world.get::<&Body>(parent).unwrap().mu };
                replace_line_path(
                    &mut self.world,
                    &app.renderer,
                    craft,
                    Some((
                        WorldPosition {
                            pos: parent_world_pos,
                        },
                        Parent { id: parent },
                        LinePathComponent::new(
                            transfer_orbit.generate_orbit_vertices(8192, parent_mu, soi_radius),
                        ),
                        AssociatedEntity { associate: craft },
                    )),
                );
                self.world.remove_one::<State>(craft).ok();
                self.world
                    .insert(craft, (transfer_orbit, Parent { id: parent }))
                    .unwrap();
            }
            Event::TakeOff { craft } => {
                let parent_id = self.world.get::<&Parent>(craft).unwrap().id;
                let parent_body_mu = self.world.get::<&Body>(parent_id).unwrap().mu;
                self.world.remove_one::<Landed>(craft).ok();
                self.world
                    .insert(
                        craft,
                        (
                            State::circular(2.0, self.current_et, parent_body_mu),
                            Parent { id: parent_id },
                        ),
                    )
                    .unwrap();
            }
            Event::Land { craft } => {
                self.world.remove_one::<State>(craft).ok();
                self.world.insert_one(craft, Landed {}).unwrap();
            }
            _ => {}
        }
    }

    /// Updates planets based on their on-rails orbits around their parent bodies
    fn orbit_system(&mut self, app: &App) {
        // Build parent -> children map
        let mut children: HashMap<Entity, Vec<Entity>> = HashMap::new();

        for (entity, (parent, _model)) in self.world.query::<(&Parent, &ModelComponent)>().iter() {
            children.entry(parent.id).or_default().push(entity);
        }

        // Collect all entities with WorldPosition
        let mut has_parent = HashMap::new();
        for (entity, parent) in self.world.query::<&Parent>().iter() {
            has_parent.insert(entity, parent.id);
        }

        // Find roots (entities without parent)
        let mut roots = Vec::new();
        for (entity, _) in self.world.query::<(&WorldPosition, &Body)>().iter() {
            if !has_parent.contains_key(&entity) {
                roots.push(entity);
            }
        }

        let et = self.current_et;

        // Kick off from roots
        for root in roots {
            let mu = { self.world.get::<&mut Body>(root).unwrap().mu };
            let root_pos = vec3(0.0, 0.0, 0.0);
            self.propagate(&children, root, root_pos, mu, et, app);
        }
    }

    fn propagate(
        &mut self,
        children: &HashMap<Entity, Vec<Entity>>,
        entity: Entity,
        parent_pos: DVec3,
        parent_mu: f64,
        t: EphemerisTime,
        app: &App,
    ) {
        // Borrow components
        let mut world_pos = self.world.get::<&mut WorldPosition>(entity).unwrap();
        let mut model = self.world.get::<&mut ModelComponent>(entity).unwrap();
        let scene_obj = self.world.get::<&SceneObject>(entity).unwrap();

        // Compute local offset if Orbit exists
        let local_offset = if let Ok(orbit) = self.world.get::<&State>(entity) {
            orbit.propagate(t, parent_mu).r
        } else {
            vec3(0.0, 0.0, 0.0)
        };

        let new_world = parent_pos + local_offset;
        let vel = new_world - world_pos.pos;
        world_pos.pos = new_world;
        model.set_position(nalgebra_glm::convert(new_world - self.camera_3d.world_pos));
        self.bvh.move_obj(
            scene_obj.bvh_node_id.unwrap(),
            &app.renderer.get_model_aabb(&model),
            &nalgebra_glm::convert(vel),
        );

        drop(world_pos); // release borrow before recusion
        drop(model); // release borrow before recusion
        drop(scene_obj); // release borrow before recusion

        // Recurse into children
        if let Some(kids) = children.get(&entity) {
            for &child in kids {
                let mu = { self.world.get::<&mut Body>(entity).unwrap().mu };
                self.propagate(children, child, new_world, mu, t, app);
            }
        }
    }

    // Updates craft to be on the surface of their planet
    fn landed_system(&mut self, app: &App) {
        // Extract out positions
        let mut pos_map = HashMap::new();
        for (entity, (world_pos, body)) in self.world.query::<(&WorldPosition, &Body)>().iter() {
            pos_map.insert(entity, (world_pos.pos, body.body_radius));
        }

        for (_entity, (world_pos, parent, _landed, scene_obj, model)) in self.world.query_mut::<(
            &mut WorldPosition,
            &Parent,
            &Landed,
            &SceneObject,
            &mut ModelComponent,
        )>() {
            let (parent_pos, parent_radius) = pos_map.get(&parent.id).unwrap();

            let new_world = parent_pos + vec3(0.0, *parent_radius, 0.0);
            let vel = new_world - world_pos.pos;
            world_pos.pos = new_world;
            model.set_position(nalgebra_glm::convert(new_world - self.camera_3d.world_pos));
            self.bvh.move_obj(
                scene_obj.bvh_node_id.unwrap(),
                &app.renderer.get_model_aabb(model),
                &nalgebra_glm::convert(vel),
            );
        }
    }

    fn select_system(&mut self) {
        if let Some(selected_entity) = self.selection.selected_entity() {
            for (entity, world_pos) in self.world.query_mut::<(&mut WorldPosition)>() {
                if entity == selected_entity {
                    self.selection.selected_pos = world_pos.pos;
                }
            }
        }
    }

    fn mouse_hover_system(&mut self, app: &App) {
        self.hovered = None;

        let mouse_pos = app.mouse_pos;
        for (entity, (world_pos, _model)) in self
            .world
            .query::<(&WorldPosition, &ModelComponent)>()
            .iter()
        {
            let relative_pos = world_pos.pos - self.camera_3d.world_pos;
            let screen_pos = self.world_to_screen(relative_pos, app);
            if screen_pos.is_none() {
                continue;
            }
            let screen_pos = screen_pos.unwrap();
            let dist = nalgebra_glm::l1_norm(&(screen_pos - mouse_pos));
            if dist < 16.0 {
                self.hovered = Some(entity);
                if app.mouse_left_clicked {
                    self.selection.set_selected(entity, app.seconds as f64);
                    self.gui = self.rebuild_gui(app);
                }
            }
        }
    }

    fn line_path_system(&mut self, app: &App) {
        // Extract out the world positions
        let mut pos_map = HashMap::new();
        for (entity, world_pos) in self.world.query::<&WorldPosition>().iter() {
            pos_map.insert(entity, world_pos.pos);
        }

        // Find which body the camera is closest to, and how close
        let mut closest_body: Option<Entity> = None;
        let mut closest_dist = f64::INFINITY;
        for (entity, (world_pos, _body)) in self.world.query::<(&WorldPosition, &Body)>().iter() {
            let dist = (world_pos.pos - self.camera_3d.world_pos).norm();
            if dist < closest_dist {
                closest_dist = dist;
                closest_body = Some(entity);
            }
        }
        let closest_body = self
            .get_ancestor(closest_body.unwrap())
            .unwrap_or(self.selection.bodies[0]);
        let closest_planet = self.get_ancestor(closest_body).unwrap_or(closest_body);
        let closest_planet_soi = {
            let closest_planet_body = self.world.get::<&Body>(closest_planet).unwrap();
            let closest_planet_orb = self.world.get::<&State>(closest_planet).unwrap();
            let sun_body = self.world.get::<&Body>(self.selection.bodies[0]).unwrap();
            sphere_of_influence(
                closest_planet_orb.semi_major_axis(SUN_MU),
                closest_planet_body.mass(),
                sun_body.mass(),
            )
        };

        // Get the associated craft, if it exists
        let mut assoc_entity_map = HashMap::new();
        for (entity, _line) in self.world.query::<&LinePathComponent>().iter() {
            assoc_entity_map.insert(
                entity,
                self.world
                    .get::<&AssociatedEntity>(entity)
                    .map_or(Entity::DANGLING, |x| x.associate),
            );
        }

        let mut mu_map = HashMap::new();
        for (entity, (_line, parent)) in self.world.query::<(&LinePathComponent, &Parent)>().iter()
        {
            let parent_entity = parent.id;
            let parent_body_mu = self.world.get::<&Body>(parent_entity).unwrap().mu;

            mu_map.insert(entity, parent_body_mu);
        }

        let mut mean_anomaly_map = HashMap::new();
        for (entity, assoc_entity) in &assoc_entity_map {
            if *assoc_entity == Entity::DANGLING {
                mean_anomaly_map.insert(entity, 0.0);
            } else {
                let assoc_state = self
                    .world
                    .get::<&State>(*assoc_entity)
                    .expect("the associated entity's gotta have state");
                let mu = *mu_map.get(entity).unwrap();

                // hyperbolic orbits don't have a meaningful mean anomaly, use 0
                if assoc_state.ecc(mu) >= 1.0 {
                    mean_anomaly_map.insert(entity, 0.0);
                } else {
                    let mean_anomaly_0 = assoc_state.mean_anomaly(mu); // M at assoc_state.t = vertex 0
                    let state_now = assoc_state.propagate(self.current_et, mu);
                    let mean_anomaly = state_now.mean_anomaly(mu);
                    mean_anomaly_map.insert(entity, mean_anomaly - mean_anomaly_0);
                }
            }
        }

        let mut proximity_alphas = HashMap::new();
        for (entity, (_line, _parent)) in self.world.query::<(&LinePathComponent, &Parent)>().iter()
        {
            let assoc_entity = *assoc_entity_map.get(&entity).unwrap();
            let assoc_planet = self
                .get_ancestor(assoc_entity)
                .unwrap_or(self.selection.bodies[0]);

            let camera_dist =
                (pos_map.get(&closest_body).unwrap() - self.camera_3d.world_pos).norm();

            // fade if:
            let fade_orbit = if assoc_entity == assoc_planet {
                // I'm a planet, and camera is close to me
                camera_dist < closest_planet_soi
            } else {
                // I'm a moon/craft, and camera is close to a planet thats not mine
                closest_planet != assoc_planet && closest_dist < closest_planet_soi
            };

            let proximity_alpha = if fade_orbit { 0.0 } else { 1.0 };

            proximity_alphas.insert(entity, proximity_alpha);
        }

        // Set the origins of the line paths wrt the parent world positions
        for (entity, (line, world_pos, parent)) in
            self.world
                .query_mut::<(&mut LinePathComponent, &mut WorldPosition, &Parent)>()
        {
            let parent_pos = pos_map.get(&parent.id).unwrap();

            let selected = match self.selection.selected_entity() {
                Some(selected_entity) => {
                    let assoc_craft = *assoc_entity_map.get(&entity).unwrap();
                    assoc_craft == selected_entity
                }
                None => false,
            };

            line.color = vec4(91.25, 160.0, 228.75, 0.0) / 255.0;

            if selected && !self.selection.is_animating(app.seconds as f64) {
                line.color.w = 0.8;
                line.width = 2.0;
            } else {
                line.color.w = 0.36606;
                line.width = 1.0;
            }

            line.color.w *= proximity_alphas.get(&entity).unwrap();

            let mean_anomaly = mean_anomaly_map.get(&entity).unwrap();
            line.seam = (mean_anomaly / (2.0 * PI)).rem_euclid(1.0) as f32;

            world_pos.pos = *parent_pos;
        }
    }

    fn get_ancestor(&self, entity: Entity) -> Option<Entity> {
        let mut child = entity;
        loop {
            let parent = self.world.get::<&Parent>(child).ok()?; // if sun, this will return None (sun has no parent)
            let parent_body = self.world.get::<&Body>(parent.id).ok()?;
            if parent_body.mu == SUN_MU {
                return Some(child);
            }
            child = parent.id;
        }
    }

    /// Updates the camera position and lookat based on mouse panning and body selection
    fn camera_update(&mut self, app: &App) {
        let rot_matrix = nalgebra_glm::rotate_y(
            &nalgebra_glm::rotate_z(&nalgebra_glm::one(), self.phi),
            self.theta,
        );
        let transition =
            cubic_ease_in_out((app.seconds as f64 - self.selection.transition).min(1.0));
        let offset = (1.0 - transition) * self.selection.prev_selected_pos
            + transition * self.selection.selected_pos;
        self.camera_3d.world_pos =
            (rot_matrix * nalgebra_glm::vec4(self.distance, 0., 0., 0.)).xyz() + offset;
        self.camera_3d.sync(offset);
    }

    fn world_to_screen(&self, relative_pos: DVec3, app: &App) -> Option<Vec2> {
        let window_size = app.window_size;
        let (view, proj) = self.camera_3d.inner.view_proj_matrices();
        let clip = proj
            * view
            * vec4(
                relative_pos.x as f32,
                relative_pos.y as f32,
                relative_pos.z as f32,
                1.0,
            );
        if clip.w <= 0.0 {
            return None;
        } // behind camera
        let ndc = clip.xyz() / clip.w;
        Some(vec2(
            ((ndc.x + 1.0) / 2.0) as f32 * window_size.x as f32,
            ((1.0 - ndc.y) / 2.0) as f32 * window_size.y as f32,
        ))
    }

    fn render_dots(&mut self, app: &App) {
        app.renderer.set_color(vec4(1.0, 1.0, 1.0, 1.0));

        for (_entity, (world_pos, _model)) in self
            .world
            .query::<(&WorldPosition, &ModelComponent)>()
            .iter()
        {
            let relative_pos = world_pos.pos - self.camera_3d.world_pos;
            if let Some(screen) = self.world_to_screen(relative_pos, app) {
                let rect = Rectangle {
                    pos: screen,
                    size: vec2(1.0, 1.0),
                };
                app.renderer.fill_rect(rect);
            }
        }
    }
}

/// Cubic easing out function - for animation
fn cubic_ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powf(3.0) / 2.0
    }
}
