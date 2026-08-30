//! This module is responsible for defining the gameplay scene.

use std::{collections::HashMap, f64::consts::PI};

use apricot::{
    app::{App, Scene},
    bvh::BVH,
    camera::{Camera, ProjectionKind},
    high_precision::{self, WorldPosition},
    opengl::create_program,
    ray::Ray,
    rectangle::Rectangle,
    render_core::{LinePathComponent, ModelComponent, RenderContext},
    shadow_map::DirectionalLightSource,
    sphere::Sphere,
};
use hecs::{Entity, World};
use nalgebra_glm::{vec2, vec3, vec4, DVec3, Vec2, Vec3};
use sdl2::keyboard::Scancode::{self};

use crate::{
    astro::{epoch::EphemerisTime, maneuver::sphere_of_influence, state::State, units::SUN_MU},
    components::{
        craft::{replace_line_path, AssociatedEntity, Command, Stage},
        factory::{spawn_factory, Factory},
        inventory::PartInventory,
        parts::PartRegistry,
        tile::{SurfaceTile, TileMap, TileSets},
        vab::{spawn_vab, Vab},
    },
    container,
    generation::lexicon::Lexicon,
    scenes::{
        events::{Event, EventQueue},
        maneuver::ManeuverModal,
        starbox::Starbox,
        vab::VabUi,
    },
    ui::{
        anchor::{Anchor, AnchorPoint},
        container::{Align, Flow},
        hrule::HRule,
        label::Label,
        progress_bar::ProgressBar,
        style::STYLE,
        text_button::TextButton,
    },
};

use crate::{
    components::{
        body::{spawn_body, Body, Category, Parent, SceneObject},
        craft::{spawn_landed_craft, Craft, Landed},
        icosphere,
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
pub const UV_DATA: &[u8] = include_bytes!("../../res/uv-sphere.obj");
pub const CONE_DATA: &[u8] = include_bytes!("../../res/cone.obj");
pub const CUBE_DATA: &[u8] = include_bytes!("../../res/cube.obj");

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
    selected_tile: Option<(Entity, usize, LinePathComponent)>,
    clicked_tile_key: Option<(Entity, usize)>,
    hovered_tile: Option<(Entity, usize, LinePathComponent)>,

    /// All the parts, loaded from the toml
    parts: PartRegistry,

    /// Up-down view angle
    phi: f64,
    /// Side-side view angle
    theta: f64,
    /// How far the camera swivels around the currently selected body
    distance: f64,

    /// Used for tab key latch
    prev_tab_state: bool,

    turn_gui: Anchor<TurnMessages>,
    gui: Anchor<CommandMessages>,
    vab_ui: VabUi,
    maneuver_ui: ManeuverModal,

    // Events and timeline
    event_queue: EventQueue,
    current_et: EphemerisTime,
    animation_start_et: EphemerisTime,
    animation_target_et: EphemerisTime,
    animation_start_real: f64,

    // Vec of unit vectors
    starbox: Starbox,
}

#[derive(Clone)]
enum TurnMessages {
    NextTurn,
}

#[derive(Clone)]
enum CommandMessages {
    FactoryCommand { part_id: String },
    OpenVab,
    OpenManeuver,
}

#[derive(Debug)]
enum SelectionKind {
    Craft,
    Body,
    Building,
}

struct SelectionState {
    pub crafts: Vec<Entity>,
    pub bodies: Vec<Entity>,
    pub buildings: Vec<Entity>,

    pub selected: Option<usize>,
    pub kind: SelectionKind,

    // For swoosh animation
    pub selected_pos: DVec3,
    pub prev_selected_pos: DVec3,
    pub transition: f64,
}

impl SelectionState {
    pub fn new(crafts: Vec<Entity>, bodies: Vec<Entity>, buildings: Vec<Entity>) -> Self {
        Self {
            crafts,
            bodies,
            buildings,
            selected: None,
            kind: SelectionKind::Body,
            selected_pos: vec3(0.0, 0.0, 0.0),
            prev_selected_pos: vec3(0.0, 0.0, 0.0),
            transition: 0.0,
        }
    }

    pub fn selected_entity(&self) -> Option<Entity> {
        self.selected.map(|s| self.curr_sel_track()[s])
    }

    pub fn set_selected(&mut self, entity: Entity, app_seconds: f64) {
        if let Some(selected) = self.selected_entity() {
            if selected == entity {
                return;
            }
        }

        let found = self
            .crafts
            .iter()
            .position(|e| *e == entity)
            .map(|x| (x, SelectionKind::Craft))
            .or(self
                .bodies
                .iter()
                .position(|e| *e == entity)
                .map(|x| (x, SelectionKind::Body)))
            .or(self
                .buildings
                .iter()
                .position(|e| *e == entity)
                .map(|x| (x, SelectionKind::Building)));

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
            SelectionKind::Building => &self.buildings,
        }
    }
}

impl Scene for Gameplay {
    /// Update the scene every tick
    fn update(&mut self, app: &App) {
        let modal_open = self.vab_ui.is_shown() || self.maneuver_ui.is_shown();

        if self.vab_ui.update(app) {
            if let Some(selected) = self.selection.selected_entity() {
                let parent = self.world.get::<&Parent>(selected).unwrap().id;

                let payload = self
                    .vab_ui
                    .payload()
                    .expect("the VAB shouldn't allow invalid payloads");

                let stages = self
                    .vab_ui
                    .stages()
                    .into_iter()
                    .collect::<Option<Vec<Stage>>>()
                    .expect("the VAB shouldn't allow invalid stages");

                {
                    let mut inventory = self.world.get::<&mut PartInventory>(parent).unwrap();
                    inventory
                        .take(&self.vab_ui.payload.clone().unwrap().id)
                        .unwrap();
                    for stage in &self.vab_ui.stages {
                        inventory.take(&stage.id).unwrap()
                    }
                }

                let landed_craft_entity = spawn_landed_craft(
                    payload,
                    stages,
                    SceneObject {
                        bvh_node_id: None,
                        name: String::from("landed craft"),
                    },
                    Parent { id: parent },
                    &mut self.world,
                    &app.renderer,
                    &mut self.bvh,
                );
                self.selection.crafts.push(landed_craft_entity);
            }
        }

        if let Some(result) = self.maneuver_ui.update(self.current_et, &self.world, app) {
            if let Some(selected) = self.selection.selected_entity() {
                let command = result.into_command();
                self.world.get::<&mut Craft>(selected).unwrap().command = Some(command);
                self.gui = self.rebuild_gui(app);
            }
        }

        if !modal_open {
            // Handle all the messages from UI
            for msg in recv_msgs(app, &mut self.gui) {
                match msg {
                    CommandMessages::FactoryCommand { part_id } => {
                        if let Some(selected) = self.selection.selected_entity() {
                            // TODO: Subtract parts from inventory
                            self.world
                                .get::<&mut Factory>(selected)
                                .unwrap()
                                .start_job(part_id, self.current_et, &self.parts)
                                .expect("you wouldnt give a fake part would you");
                            self.gui = self.rebuild_gui(app);
                        }
                    }
                    CommandMessages::OpenVab => {
                        if let Some(selected) = self.selection.selected_entity() {
                            let parent = self.world.get::<&Parent>(selected).unwrap().id;
                            let inventory = self.world.get::<&PartInventory>(parent).unwrap();
                            self.vab_ui.show(&inventory, &self.parts, app);
                        }
                    }
                    CommandMessages::OpenManeuver => {
                        if let Some(selected) = self.selection.selected_entity() {
                            self.maneuver_ui
                                .show(selected, self.current_et, &self.world, app);
                        }
                    }
                }
            }

            for msg in recv_msgs(app, &mut self.turn_gui) {
                match msg {
                    TurnMessages::NextTurn => {
                        if !self.is_animating() {
                            self.schedule_events();
                            // Handle any events already at the current time before advancing
                            let due_now = self.event_queue.pop_due(self.current_et);
                            for event in due_now {
                                self.handle_event(event, app);
                            }
                            if let Some((&next_event_time, _)) =
                                self.event_queue.events.iter().next()
                            {
                                self.animation_start_et = self.current_et;
                                self.animation_target_et = next_event_time;
                                self.animation_start_real = app.seconds as f64;
                            }
                        }
                    }
                }
            }
        }

        if self.is_animating() {
            const TURN_TIME: f64 = 0.75;
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
                self.turn_gui = self.rebuild_turn_gui(app);
            }
        }

        self.control(app);
        self.orbit_system();
        self.landed_system();
        self.select_system();
        self.camera_update(app);
        if !modal_open {
            self.hovered = None;
            self.mouse_hover_system(app, false);
            self.tile_select_system(app);
            self.mouse_hover_system(app, true);
        }
        self.sync_selected_tile(app);
        self.line_path_system(app);
        self.sync_models(app);

        // Delete anything we want deleted
        app.renderer.flush_deletion_queue();
    }

    /// Render the scene to the screen when time allows
    fn render(&mut self, app: &App) {
        // Set everything up
        let aspect = app.window_size.x as f32 / app.window_size.y as f32;
        if (self.camera_3d.inner.aspect_ratio() - aspect).abs() > 1e-6 {
            self.camera_3d.inner.set_aspect_ratio(aspect);
        }

        self.directional_light.light_dir =
            -nalgebra_glm::convert::<DVec3, Vec3>(self.camera_3d.world_pos);
        app.renderer.set_camera(self.camera_3d.inner);
        let font = app.renderer.get_font_id_from_name("font").unwrap();
        app.renderer.set_font(font);

        // Draw the 3D stuff
        app.renderer.set_color(vec4(0.01, 0.01, 0.01, 1.0));
        app.renderer.clear();
        self.starbox.draw(app);
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
        if let Some(entity) = self.selection.selected_entity() {
            if !self.selection.is_animating(app.seconds as f64)
                && self.world.get::<&Craft>(entity).is_ok()
            {
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
                    &vec4(1.0, 1.0, 1.0, 1.0),
                );
            }
        }

        // Draw hovered reticle
        if let (Some(hovered), Some(selected)) = (self.hovered, self.selection.selected_entity()) {
            if hovered != selected {
                let hovered_world_pos = self.world.get::<&WorldPosition>(hovered).unwrap().pos;
                let scene_obj = self.world.get::<&SceneObject>(hovered).unwrap();

                let radius = self
                    .world
                    .get::<&Body>(hovered)
                    .map(|b| b.body_radius)
                    .unwrap_or(0.0);

                let relative_pos = hovered_world_pos - self.camera_3d.world_pos;
                match self.world_to_screen(relative_pos, app) {
                    Some(screen_pos)
                        if self.apparent_radius_px(radius, relative_pos.norm(), app) < 2.0
                            && !self.is_occluded(hovered, relative_pos) =>
                    {
                        let width = 16.0;

                        let reticle_texture =
                            app.renderer.get_texture_id_from_name("reticle").unwrap();
                        app.renderer.copy_texture(
                            Rectangle::new(
                                screen_pos.x - width * 0.5,
                                screen_pos.y - width * 0.5,
                                width,
                                width,
                            ),
                            reticle_texture,
                            Rectangle::new(0.0, 0.0, 16.0, 16.0),
                            &vec4(1.0, 1.0, 1.0, 1.0),
                        );
                        app.renderer
                            .draw_text(screen_pos + vec2(8.0, 8.0), &scene_obj.name);
                    }
                    _ => {}
                };
            }
        }

        for (selected, _, line_path) in [&self.selected_tile, &self.hovered_tile]
            .into_iter()
            .flatten()
        {
            let world_pos = self.world.get::<&WorldPosition>(*selected).unwrap().pos;
            let relative_pos = world_pos - self.camera_3d.world_pos;
            let (view_matrix, proj_matrix) = self.camera_3d.inner.view_proj_matrices();

            app.renderer.draw_line_path_at(
                line_path,
                nalgebra_glm::convert(relative_pos),
                view_matrix,
                proj_matrix,
            );
        }

        // Draw GUI
        self.gui.render(app);
        self.turn_gui.render(app);
        self.vab_ui.render(app);
        self.maneuver_ui.render(app);
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
        app.renderer.add_program(
            create_program(
                include_str!("../shaders/starbox.vert"),
                include_str!("../shaders/starbox.frag"),
            )
            .unwrap(),
            Some("starbox"),
        );

        // Setup the mesh manager
        app.renderer
            .add_mesh_from_obj(QUAD_XY_DATA, Some("quad-xy"));
        app.renderer.add_mesh_from_obj(UV_DATA, Some("uv"));
        app.renderer.add_mesh_from_obj(CONE_DATA, Some("cone"));
        app.renderer.add_mesh_from_obj(CUBE_DATA, Some("cube"));

        let ico_20 = icosphere::generate(0); // 20-face icosphere for dwarf bodies
        let ico_80 = icosphere::generate(1); // 80-face icosphere for mars-like sub-earths
        let ico_320 = icosphere::generate(2); // 320-face icosphere for large rocky bodies
        app.renderer.add_mesh_from_verts(
            ico_20.indices.clone(),
            vec![&ico_20.positions, &ico_20.normals, &ico_20.uvs],
            Some("ico-20"),
        );
        app.renderer.add_mesh_from_verts(
            ico_80.indices.clone(),
            vec![&ico_80.positions, &ico_80.normals, &ico_80.uvs],
            Some("ico-80"),
        );
        app.renderer.add_mesh_from_verts(
            ico_320.indices.clone(),
            vec![&ico_320.positions, &ico_320.normals, &ico_320.uvs],
            Some("ico-320"),
        );
        let tile_sets = TileSets {
            dwarf: ico_20.tile_tris,
            sub: ico_80.tile_tris,
            large: ico_320.tile_tris,
        };

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
            .add_font("res/Consolas.ttf", "font", 15, sdl2::ttf::FontStyle::NORMAL);
        app.renderer.add_font(
            "res/Consolas.ttf",
            "font-small-bold",
            16,
            sdl2::ttf::FontStyle::BOLD,
        );
        app.renderer.add_font(
            "res/Consolas.ttf",
            "font-small-italic",
            16,
            sdl2::ttf::FontStyle::ITALIC,
        );
        app.renderer.add_font(
            "res/Consolas.ttf",
            "font-big",
            21,
            sdl2::ttf::FontStyle::BOLD,
        );

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
            &tile_sets,
            &mut world,
            &app.renderer,
            &mut bvh,
        );

        let mut bodies = vec![sun_entity];
        let crafts = vec![];
        let mut buildings = vec![];

        let (_lexicon, _node_count) = Lexicon::create("res/names.txt", "res/names.lex");
        let lexicon = Lexicon::read("res/names.lex");

        let parts = PartRegistry::load_from_dir("res/parts");

        let mut habitable_planet = 0;
        let mut num_planets = 0;
        let planets = solar_system_gen::generate();
        for system in planets {
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
                &tile_sets,
                &mut world,
                &app.renderer,
                &mut bvh,
            );
            num_planets += 1;
            if num_planets == 3 {
                habitable_planet = bodies.len();
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
                    &tile_sets,
                    &mut world,
                    &app.renderer,
                    &mut bvh,
                );
                bodies.push(moon_entity);
            }
        }

        let factory = spawn_factory(
            SceneObject {
                bvh_node_id: None,
                name: String::from("factory"),
            },
            Parent {
                id: bodies[habitable_planet],
            },
            0,
            &mut world,
            &app.renderer,
            &mut bvh,
        );
        buildings.push(factory);

        let vab = spawn_vab(
            SceneObject {
                bvh_node_id: None,
                name: String::from("vehicle assembly building"),
            },
            Parent {
                id: bodies[habitable_planet],
            },
            3,
            &mut world,
            &app.renderer,
            &mut bvh,
        );
        buildings.push(vab);

        let gui = Anchor::<CommandMessages>::new(
            Box::new(container![].at(vec2(100.0, 100.0))),
            AnchorPoint::CenterRight,
        );

        let turn_gui = Anchor::<TurnMessages>::new(
            Box::new(container![
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
                .on_click(TurnMessages::NextTurn),
                TextButton::new(Rectangle::new(100.0, 120.0, 200.0, 30.0,), "Click me!")
                    .use_style(&STYLE)
                    .border(STYLE.border_primary, 1.0)
                    .on_click(TurnMessages::NextTurn),
            ]),
            AnchorPoint::BottomRight,
        );

        let font = app.renderer.get_font_id_from_name("font").unwrap();
        app.renderer.set_font(font);

        let mut event_queue = EventQueue::new();
        event_queue.push(
            EphemerisTime::epoch() + EphemerisTime::from_days(30.0),
            Event::Background,
        );

        Self {
            world,
            camera_3d: high_precision::Camera {
                world_pos: vec3(1.0, 1.0, 1.0),
                inner: Camera::new(
                    vec3(1.0, 0.0, 1.0),
                    vec3(0.0, 0.0, 0.0),
                    vec3(0.0, 0.0, 1.0),
                    ProjectionKind::Perspective {
                        fov_rad: 37.0f32.to_radians(),
                        far: 10000000.0,
                    },
                    4.0 / 3.0,
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
                    4.0 / 3.0,
                ),
                vec3(-1.0, 0.0, 0.0),
                1024,
            ),

            selection: SelectionState::new(crafts, bodies, buildings),
            hovered: None,
            selected_tile: None,
            clicked_tile_key: None,
            hovered_tile: None,

            parts,

            phi: 2.5,
            theta: -PI / 4.0,
            distance: 20.0,
            prev_tab_state: false,

            gui,
            turn_gui,
            vab_ui: VabUi::new(),
            maneuver_ui: ManeuverModal::new(),

            current_et: EphemerisTime::epoch(),
            animation_start_et: EphemerisTime::epoch(),
            animation_target_et: EphemerisTime::epoch(),
            animation_start_real: 0.0,
            event_queue,

            starbox: Starbox::new(9000, vec3(1.0, 2.0, 4.0), 0.4),
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

        let body_radius = self.get_selected_body_radius().unwrap_or(0.0);
        let altitude = self.distance - body_radius;

        let min_distance: f64 = 0.12 + body_radius;
        let max_distance: f64 = 1e6 + body_radius;

        let control_speed = 0.0005 * (altitude - min_distance).clamp(4.0, 10.0);
        if app.mouse_left_dragging {
            self.phi -= control_speed * (app.mouse_vel.x as f64);
            self.theta = (self.theta - control_speed * (app.mouse_vel.y as f64))
                .max(control_speed - PI / 2.0)
                .min(PI / 2.0 - control_speed);
        }

        let zoom_factor = 0.9f64.powf(app.mouse_wheel as f64);

        self.distance = (self.distance * zoom_factor).clamp(min_distance, max_distance);
    }

    fn rebuild_gui(&self, app: &App) -> Anchor<CommandMessages> {
        let mut widgets: Vec<Box<dyn Widget<CommandMessages>>> = vec![];
        let selected = self.selection.selected_entity();
        if let Some(selected) = selected {
            widgets.extend(self.build_selection_widgets(selected, app));
        }

        let mut anchor = Anchor::new(
            Box::new(
                Container::new(widgets)
                    .cross_align(Align::Start)
                    .background_color(STYLE.bg_primary)
                    .border(STYLE.border_primary, 1.0)
                    .padding(vec2(8.0, 8.0)),
            ),
            AnchorPoint::TopRight,
        )
        .margin(vec2(16.0, 16.0));
        anchor.reposition(app);
        anchor
    }

    fn rebuild_turn_gui(&mut self, app: &App) -> Anchor<TurnMessages> {
        let mut turn_widgets: Vec<Box<dyn Widget<TurnMessages>>> = vec![];
        turn_widgets.extend(self.build_footer_widgets(app));

        let mut anchor = Anchor::new(
            Box::new(Container::new(turn_widgets).cross_align(Align::End)),
            AnchorPoint::BottomRight,
        );
        anchor.reposition(app);
        anchor
    }

    fn build_footer_widgets(&self, app: &App) -> Vec<Box<dyn Widget<TurnMessages>>> {
        let font = app.renderer.get_font_id_from_name("font").unwrap();

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
                .on_click(TurnMessages::NextTurn),
            ),
            Box::new(Label::new(format!("ET: {}", self.current_et.as_calendar())).font(font, app)),
        ]
    }

    fn build_selection_widgets(
        &self,
        selected: Entity,
        app: &App,
    ) -> Vec<Box<dyn Widget<CommandMessages>>> {
        let mut widgets: Vec<Box<dyn Widget<CommandMessages>>> = vec![];

        if self.world.get::<&Craft>(selected).is_ok() {
            widgets.extend(self.build_craft_info(selected, app));
        } else if self.world.get::<&Body>(selected).is_ok() {
            widgets.extend(self.build_body_info(selected, app));
        } else if self.world.get::<&Factory>(selected).is_ok() {
            widgets.extend(self.build_factory_info(selected, app));
        } else if self.world.get::<&Vab>(selected).is_ok() {
            widgets.extend(self.build_vab_info(selected, app));
        }

        widgets
    }

    fn build_craft_info(
        &self,
        selected: Entity,
        app: &App,
    ) -> Vec<Box<dyn Widget<CommandMessages>>> {
        const WIDTH: f32 = 280.0;
        let font = app.renderer.get_font_id_from_name("font").unwrap();
        let font_small_bold = app
            .renderer
            .get_font_id_from_name("font-small-bold")
            .unwrap();
        let font_small_italic = app
            .renderer
            .get_font_id_from_name("font-small-italic")
            .unwrap();
        let font_big = app.renderer.get_font_id_from_name("font-big").unwrap();

        let craft = self.world.get::<&Craft>(selected).unwrap();
        let scene_object = self.world.get::<&SceneObject>(selected).unwrap();

        let craft_dv = craft.total_remaining_dv();

        let is_idle = craft.command.is_none();

        let mut widgets: Vec<Box<dyn Widget<CommandMessages>>> = vec![
            Box::new(Label::new(scene_object.name.clone().to_uppercase()).font(font_big, app)),
            Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)),
            Box::new(Label::new("MISSION").font(font_small_bold, app)),
        ];

        if let Some(command) = &craft.command {
            widgets.push(Box::new(
                Label::new(command.label().to_uppercase())
                    .font(font_small_bold, app)
                    .color(STYLE.accent),
            ));
            for (burn_label, et) in command.burn_schedule() {
                let done = self.current_et >= et;
                widgets.push(Box::new(
                    Container::new(vec![
                        Box::new(Label::new(burn_label).font(font_small_bold, app).color(
                            if done {
                                STYLE.positive
                            } else {
                                STYLE.text_primary
                            },
                        )),
                        Box::new(Label::new(et.as_calendar()).font(font, app).color(if done {
                            STYLE.positive
                        } else {
                            STYLE.text_disabled
                        })),
                    ])
                    .flow(Flow::Vertical)
                    .border(STYLE.border_primary, 1.0)
                    .fixed_width(vec2(WIDTH, 10.0))
                    .padding(vec2(8.0, 8.0)),
                ));
            }
        } else {
            widgets.push(Box::new(
                Label::new("NO MISSION ASSIGNED")
                    .font(font_small_italic, app)
                    .color(STYLE.text_disabled),
            ));
        }

        if is_idle {
            widgets.push(Box::new(
                TextButton::<CommandMessages>::new(
                    Rectangle::new(0.0, 0.0, WIDTH, 30.0),
                    "Plan Mission...",
                )
                .use_style_accented(&STYLE)
                .on_click(CommandMessages::OpenManeuver),
            ))
        };
        widgets.push(Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)));

        // Stages from bottom to top
        widgets.push(Box::new(Label::new("STAGES").font(font_small_bold, app)));

        widgets.push(Box::new(
            Label::new(format!("Total dv: {:.0} m/s", craft_dv))
                .font(font, app)
                .color(STYLE.text_primary),
        ));

        for stage in craft.stages_stack.iter() {
            let fuel_pct = stage.fuel_mass / stage.max_fuel_mass;
            widgets.push(Box::new(
                Container::new(vec![
                    Box::new(Label::new(stage.name.clone()).font(font_small_bold, app)),
                    Box::new(
                        Label::new(format!(
                            "{:.0}/{:.0} kg",
                            stage.fuel_mass, stage.max_fuel_mass
                        ))
                        .font(font, app)
                        .color(if fuel_pct > 0.25 {
                            STYLE.text_primary
                        } else {
                            STYLE.warning
                        }),
                    ),
                    Box::new(
                        ProgressBar::new(vec2(WIDTH - 24.0, 8.0))
                            .background_color(STYLE.bg_primary)
                            .fill_color(if fuel_pct > 0.25 {
                                STYLE.accent
                            } else {
                                STYLE.warning
                            })
                            .border(STYLE.border_primary, 1.0)
                            .progress(fuel_pct as f32),
                    ),
                ])
                .flow(Flow::Vertical)
                .border(STYLE.border_primary, 1.0)
                .fixed_width(vec2(WIDTH, 10.0)),
            ));
        }

        widgets
    }

    fn build_body_info(
        &self,
        selected: Entity,
        app: &App,
    ) -> Vec<Box<dyn Widget<CommandMessages>>> {
        let font = app.renderer.get_font_id_from_name("font").unwrap();
        let scene_object = self.world.get::<&SceneObject>(selected).unwrap();
        let body = self.world.get::<&Body>(selected).unwrap();
        let inventory = self.world.get::<&PartInventory>(selected).unwrap();

        // let state = self.world.get::<&State>(selected).unwrap();
        // Know: name, radius, mass, density, orbital radius, rotation in hours
        // Have to find: atmos press, temp, core mass fraction, magnetic field
        let mut widgets: Vec<Box<dyn Widget<CommandMessages>>> = vec![
            Box::new(
                Label::new(format!("NAME:\n  {}\n", scene_object.name.clone())).font(font, app),
            ),
            Box::new(
                Label::new(format!("EARTH RADII:\n  {:.1}\n", body.body_radius)).font(font, app),
            ),
            Box::new(Label::new(format!("EARTH MASSES:\n  {:.3}\n", body.mass())).font(font, app)),
            Box::new(
                Label::new(format!("DENSITY (g/cm^3):\n  {:.1}\n", body.density)).font(font, app),
            ),
            Box::new(
                Label::new(format!(
                    "DAY (hours):\n  {:.1}\n",
                    body.rotation_period_hours
                ))
                .font(font, app),
            ),
            Box::new(
                Label::new(format!(
                    "SURFACE PRESSURE:\n  {:.1} bar\n",
                    body.atmos_pressure
                ))
                .font(font, app),
            ),
            Box::new(
                Label::new(format!(
                    "SURFACE TEMPERATURE:\n  {:.0} K\n",
                    body.temperature
                ))
                .font(font, app),
            ),
            Box::new(
                Label::new(format!("CMF\n  {:.0}%\n", body.core_mass_fraction * 100.0))
                    .font(font, app),
            ),
            Box::new(
                Label::new(format!(
                    "MAGNETIC FIELD:\n  {}\n",
                    if body.magnetic_field {
                        "present"
                    } else {
                        "absent"
                    }
                ))
                .font(font, app),
            ),
        ];

        // Extend with inventory info
        widgets.extend(inventory.parts.iter().filter_map(|(part_id, quantity)| {
            if *quantity > 0 {
                Some(
                    Box::new(Label::new(format!("{}: {}", part_id, quantity)).font(font, app))
                        as Box<dyn Widget<CommandMessages>>,
                )
            } else {
                None
            }
        }));

        widgets
    }

    fn build_factory_info(
        &self,
        selected: Entity,
        app: &App,
    ) -> Vec<Box<dyn Widget<CommandMessages>>> {
        const WIDTH: f32 = 280.0;

        let font = app.renderer.get_font_id_from_name("font").unwrap();
        let font_small_bold = app
            .renderer
            .get_font_id_from_name("font-small-bold")
            .unwrap();
        let font_big = app.renderer.get_font_id_from_name("font-big").unwrap();
        let factory = self.world.get::<&Factory>(selected).unwrap();

        let mut widgets: Vec<Box<dyn Widget<CommandMessages>>> = vec![
            Box::new(Label::new("FACTORY").font(font_big, app)),
            Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)),
        ];

        if let Some(job) = &factory.current_job {
            let part = self
                .parts
                .get(&job.part_id)
                .expect("should be a valid part");

            widgets.extend(vec![
                Box::new(Label::new("STATUS").font(font_small_bold, app)),
                Box::new(Label::new(format!("Building: {}", part.name)).font(font, app))
                    as Box<dyn Widget<CommandMessages>>,
                Box::new(
                    ProgressBar::new(vec2(WIDTH, 12.0))
                        .background_color(STYLE.bg_primary)
                        .fill_color(STYLE.accent)
                        .border(STYLE.border_primary, 1.0)
                        .progress(job.progress(self.current_et) as f32),
                ) as Box<dyn Widget<CommandMessages>>,
                Box::new(
                    Label::new(format!("Completion: {}", job.completion_et.as_calendar()))
                        .font(font, app),
                ) as Box<dyn Widget<CommandMessages>>,
            ])
        } else {
            widgets.extend(vec![
                Box::new(Label::new("STATUS").font(font_small_bold, app)),
                Box::new(Label::new("No orders").font(font, app))
                    as Box<dyn Widget<CommandMessages>>,
                Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH))
                    as Box<dyn Widget<CommandMessages>>,
                Box::new(Label::new("BUILD").font(font_small_bold, app))
                    as Box<dyn Widget<CommandMessages>>,
            ]);
            let build_orders: Vec<Box<dyn Widget<CommandMessages>>> = self
                .parts
                .all()
                // TODO: Filter against what this factory can build, SWAPC wise
                .map(|part| {
                    let can_afford = true; // TODO: Check against inventory's parts
                    Box::new(
                        Container::new(vec![
                            Box::new(
                                Container::new(vec![Box::new(
                                    Label::new(part.name.clone())
                                        .font(font_small_bold, app)
                                        .color(if can_afford {
                                            STYLE.text_primary
                                        } else {
                                            STYLE.text_disabled
                                        }),
                                )])
                                .flow(Flow::Vertical)
                                .padding(vec2(0.0, 4.0))
                                .fixed_width(vec2(WIDTH * 0.8 - 24.0, 10.0)),
                            ),
                            Box::new(
                                Container::new(vec![Box::new(
                                    TextButton::<CommandMessages>::new(
                                        Rectangle::new(0.0, 0.0, 45.0, 25.0),
                                        "BUILD",
                                    )
                                    .use_style(&STYLE)
                                    .on_click(CommandMessages::FactoryCommand {
                                        part_id: part.id.clone(),
                                    })
                                    .active(can_afford),
                                )])
                                .padding(vec2(0.0, 0.0))
                                .fixed_width(vec2(WIDTH * 0.2, 10.0))
                                .cross_align(Align::End)
                                .flow(Flow::Vertical),
                            ),
                        ])
                        .border(STYLE.border_primary, 1.0)
                        .cross_align(Align::Center)
                        .flow(Flow::Horizontal),
                    ) as Box<dyn Widget<CommandMessages>>
                })
                .collect();
            widgets.push(Box::new(
                Container::new(build_orders).padding(vec2(0.0, 0.0)),
            ));
        }

        widgets
    }

    fn build_vab_info(&self, selected: Entity, app: &App) -> Vec<Box<dyn Widget<CommandMessages>>> {
        const WIDTH: f32 = 280.0;

        let font = app.renderer.get_font_id_from_name("font").unwrap();
        let font_small_bold = app
            .renderer
            .get_font_id_from_name("font-small-bold")
            .unwrap();
        let font_big = app.renderer.get_font_id_from_name("font-big").unwrap();

        let parent = self.world.get::<&Parent>(selected).unwrap().id;
        let inventory = self.world.get::<&PartInventory>(parent).unwrap();

        let mut widgets: Vec<Box<dyn Widget<CommandMessages>>> = vec![
            Box::new(Label::new("VAB").font(font_big, app)),
            Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)),
            Box::new(Label::new("INVENTORY").font(font_small_bold, app)),
        ];

        // Show available parts
        let inventory_rows: Vec<Box<dyn Widget<CommandMessages>>> = self
            .parts
            .all()
            .filter_map(|part| {
                let count = inventory.parts.get(&part.id).copied().unwrap_or(0);
                if count == 0 {
                    return None;
                }
                Some(Box::new(
                    Container::new(vec![
                        Box::new(
                            Container::new(vec![Box::new(
                                Label::new(part.name.clone()).font(font_small_bold, app),
                            )])
                            .flow(Flow::Vertical)
                            .padding(vec2(0.0, 0.0))
                            .fixed_width(vec2(WIDTH * 0.8 - 24.0, 10.0)),
                        ),
                        Box::new(
                            Container::new(vec![Box::new(
                                Label::new(format!("x{count}")).font(font, app),
                            )])
                            .padding(vec2(0.0, 0.0))
                            .fixed_width(vec2(WIDTH * 0.2, 10.0))
                            .cross_align(Align::End)
                            .flow(Flow::Vertical),
                        ),
                    ])
                    .border(STYLE.border_primary, 1.0)
                    .cross_align(Align::Center)
                    .flow(Flow::Horizontal),
                ) as Box<dyn Widget<CommandMessages>>)
            })
            .collect();

        if inventory_rows.is_empty() {
            widgets.push(Box::new(
                Label::new("No parts available")
                    .font(font, app)
                    .color(STYLE.text_disabled),
            ));
        } else {
            widgets.push(Box::new(
                Container::new(inventory_rows).padding(vec2(0.0, 0.0)),
            ));
        }

        widgets.push(Box::new(HRule::new(STYLE.border_primary, 1.0, WIDTH)));
        widgets.push(Box::new(Label::new("ASSEMBLE").font(font_small_bold, app)));
        widgets.push(Box::new(
            TextButton::<CommandMessages>::new(
                Rectangle::new(0.0, 0.0, WIDTH, 30.0),
                "Stack New Vehicle...",
            )
            .use_style_accented(&STYLE)
            .on_click(CommandMessages::OpenVab)
            .active(!inventory.parts.is_empty()),
        ));

        widgets
    }

    fn schedule_events(&mut self) {
        let crafts_with_commands: Vec<(Entity, Command)> = self
            .world
            .query::<(&mut Craft,)>()
            .iter()
            .filter_map(|(entity, (craft,))| {
                if craft.command.is_some() && !craft.command_scheduled {
                    craft.command_scheduled = true;
                    craft.command.as_ref().map(|cmd| (entity, cmd.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (entity, command) in crafts_with_commands {
            match command {
                Command::Transfer { to, plan } => {
                    let departure_time = plan.transfer_state.t;
                    let arrival_time = plan.flyby_state.t;
                    let circ_time = plan.circ_state.t;

                    println!("departure_time: {}", departure_time.as_calendar());
                    println!("arrival_time.t: {}", arrival_time.as_calendar());
                    println!("circ_time.t: {}", circ_time.as_calendar());

                    assert!(departure_time < arrival_time);
                    assert!(arrival_time < circ_time);

                    self.event_queue.push(
                        departure_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.transfer_state,
                            soi_radius: Some(plan.soi_radius * 1.1),
                            dv: plan.transfer_dv,
                        },
                    );

                    self.event_queue.push(
                        arrival_time,
                        Event::SoiChange {
                            craft: entity,
                            new_parent: to,
                            new_craft_orbit: plan.flyby_state,
                            new_soi_radius: plan.soi_radius * 3.0,
                        },
                    );

                    self.event_queue.push(
                        circ_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.circ_state,
                            soi_radius: Some(plan.soi_radius * 1.1),
                            dv: plan.circ_dv,
                        },
                    );

                    self.event_queue
                        .push(circ_time, Event::CompleteCommand { craft: entity });
                }
                Command::Flyby { to, plan } => {
                    let old_parent = self.world.get::<&Parent>(entity).unwrap().id;

                    let departure_time = plan.transfer_state.t;
                    let arrival_time = plan.flyby_state.t;
                    let exit_time = plan.exit_state.t;

                    println!("departure_time: {}", departure_time.as_calendar());
                    println!("arrival_time.t: {}", arrival_time.as_calendar());
                    println!("exit_time.t: {}", exit_time.as_calendar());

                    assert!(departure_time < arrival_time);
                    assert!(arrival_time < exit_time);

                    self.event_queue.push(
                        departure_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.transfer_state,
                            soi_radius: Some(plan.soi_radius),
                            dv: plan.transfer_dv,
                        },
                    );

                    self.event_queue.push(
                        arrival_time,
                        Event::SoiChange {
                            craft: entity,
                            new_parent: to,
                            new_craft_orbit: plan.flyby_state,
                            new_soi_radius: plan.soi_radius,
                        },
                    );

                    self.event_queue.push(
                        exit_time,
                        Event::SoiChange {
                            craft: entity,
                            new_parent: old_parent,
                            new_craft_orbit: plan.exit_state,
                            new_soi_radius: plan.soi_radius,
                        },
                    );

                    self.event_queue
                        .push(exit_time, Event::CompleteCommand { craft: entity });
                }
                Command::Escape { to, plan } => {
                    let departure_time = plan.escape_burn.t;
                    let arrival_time = plan.grandparent_orbit.t;

                    println!("departure_time: {}", departure_time.as_calendar());
                    println!("arrival_time.t: {}", arrival_time.as_calendar());

                    assert!(departure_time < arrival_time);

                    self.event_queue.push(
                        departure_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.escape_burn,
                            soi_radius: Some(plan.soi_radius * 1.1),
                            dv: plan.escape_dv,
                        },
                    );

                    self.event_queue.push(
                        arrival_time,
                        Event::SoiChange {
                            craft: entity,
                            new_parent: to,
                            new_craft_orbit: plan.grandparent_orbit,
                            new_soi_radius: plan.soi_radius * 3.0,
                        },
                    );

                    self.event_queue
                        .push(arrival_time, Event::CompleteCommand { craft: entity });
                }
                Command::Launch { plan } => {
                    let launch_time = plan.launch_burn.t;
                    let circ_time = plan.circ_burn.t;

                    println!("launch_time: {}", launch_time.as_calendar());
                    println!("circ_time.t: {}", circ_time.as_calendar());

                    assert!(launch_time < circ_time);

                    self.event_queue
                        .push(launch_time, Event::Launch { craft: entity });

                    self.event_queue.push(
                        launch_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.launch_burn,
                            soi_radius: None,
                            dv: plan.launch_dv,
                        },
                    );

                    self.event_queue.push(
                        circ_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.circ_burn,
                            soi_radius: None,
                            dv: plan.circ_dv,
                        },
                    );

                    self.event_queue
                        .push(circ_time, Event::CompleteCommand { craft: entity });
                }
                Command::Land { plan } => {
                    let deorbit_time = plan.deorbit_burn.t;
                    let land_time = plan.landing_burn.t;

                    println!("deorbit_time: {}", deorbit_time.as_calendar());
                    println!("land_time.t: {}", land_time.as_calendar());

                    assert!(deorbit_time < land_time);

                    self.event_queue.push(
                        deorbit_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.deorbit_burn,
                            soi_radius: None,
                            dv: plan.deorbit_dv,
                        },
                    );
                    self.event_queue.push(
                        land_time,
                        Event::Burn {
                            craft: entity,
                            new_orbit: plan.landing_burn,
                            soi_radius: None,
                            dv: plan.landing_dv,
                        },
                    );
                    self.event_queue
                        .push(land_time, Event::Land { craft: entity });
                    self.event_queue
                        .push(land_time, Event::CompleteCommand { craft: entity });
                }
            }
        }

        let factories: Vec<(Entity, String, EphemerisTime)> = self
            .world
            .query::<(&mut Factory,)>()
            .iter()
            .filter_map(|(entity, (factory,))| {
                if let Some(job) = &mut factory.current_job {
                    if !job.scheduled {
                        job.scheduled = true;
                        return Some((entity, job.part_id.clone(), job.completion_et));
                    }
                }
                None
            })
            .collect();

        for (entity, part_id, completion_et) in factories {
            self.event_queue.push(
                completion_et,
                Event::FactoryComplete {
                    factory: entity,
                    part_id,
                },
            );
        }
    }

    fn handle_event(&mut self, event: Event, app: &App) {
        match event {
            Event::SoiChange {
                craft,
                new_parent,
                new_craft_orbit,
                new_soi_radius,
            } => {
                self.selection.set_selected(craft, app.seconds as f64);

                let new_parent_world_pos =
                    self.world.get::<&WorldPosition>(new_parent).unwrap().pos;
                let new_parent_mu = self.world.get::<&Body>(new_parent).unwrap().mu;

                replace_line_path(
                    &mut self.world,
                    &app.renderer,
                    craft,
                    Some((
                        WorldPosition {
                            pos: new_parent_world_pos, // center the orbit line path about the new parent
                        },
                        Parent { id: new_parent },
                        LinePathComponent::new(
                            new_craft_orbit
                                .generate_orbit_vertices(8192, new_parent_mu, Some(new_soi_radius))
                                .unwrap(),
                        ),
                        AssociatedEntity { associate: craft },
                    )),
                );
                self.world.remove_one::<State>(craft).ok();
                self.world
                    .insert(craft, (new_craft_orbit, Parent { id: new_parent }))
                    .unwrap();
            }
            Event::Burn {
                craft,
                new_orbit,
                soi_radius,
                dv,
            } => {
                self.selection.set_selected(craft, app.seconds as f64);

                println!(
                    "Burn firing, r={:?} v={:?} at {}",
                    new_orbit.r,
                    new_orbit.v,
                    self.current_et.as_calendar()
                );
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
                            new_orbit
                                .generate_orbit_vertices(8192, parent_mu, soi_radius)
                                .unwrap(),
                        ),
                        AssociatedEntity { associate: craft },
                    )),
                );
                {
                    let mut craft_component = self.world.get::<&mut Craft>(craft).unwrap();
                    craft_component.burn(dv);
                }
                self.world.remove_one::<State>(craft).ok();
                self.world
                    .insert(craft, (new_orbit, Parent { id: parent }))
                    .unwrap();
            }
            Event::Launch { craft } => {
                self.selection.set_selected(craft, app.seconds as f64);

                println!(
                    "Launch event firing for {:?} at {}",
                    craft,
                    self.current_et.as_calendar()
                );
                let parent_id = self.world.get::<&Parent>(craft).unwrap().id;
                self.world.remove_one::<Landed>(craft).ok();
                self.world
                    .insert(craft, (Parent { id: parent_id },))
                    .unwrap();
            }
            Event::Land { craft } => {
                self.selection.set_selected(craft, app.seconds as f64);

                let offset = {
                    let craft_state = self.world.get::<&State>(craft).unwrap();
                    let parent_id = self.world.get::<&Parent>(craft).unwrap().id;
                    let parent_body_mu = self.world.get::<&Body>(parent_id).unwrap().mu;
                    craft_state
                        .propagate(self.current_et, parent_body_mu)
                        .unwrap()
                        .r
                };

                self.world.remove_one::<State>(craft).ok();
                replace_line_path(&mut self.world, &app.renderer, craft, None);
                self.world.insert_one(craft, Landed { offset }).unwrap();
            }
            Event::CompleteCommand { craft } => {
                let mut craft = self.world.get::<&mut Craft>(craft).unwrap();
                craft.command = None;
                craft.command_scheduled = false;
            }
            Event::FactoryComplete { factory, part_id } => {
                self.selection.set_selected(factory, app.seconds as f64);

                let parent = self.world.get::<&Parent>(factory).unwrap().id;
                let mut part_inventory = self.world.get::<&mut PartInventory>(parent).unwrap();
                part_inventory.add(part_id.as_str(), 1);

                // clear job so that factory becomes idle
                if let Ok(mut f) = self.world.get::<&mut Factory>(factory) {
                    f.current_job = None;
                }
            }
            Event::Background => {
                // Schedule the next quarterly payout
                self.event_queue.push(
                    self.current_et + EphemerisTime::from_days(30.0),
                    Event::Background,
                );
            }
        }
    }

    /// Updates planets based on their on-rails orbits around their parent bodies
    fn orbit_system(&mut self) {
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
            self.propagate(&children, root, root_pos, mu, et);
        }
    }

    fn propagate(
        &mut self,
        children: &HashMap<Entity, Vec<Entity>>,
        entity: Entity,
        parent_pos: DVec3,
        parent_mu: f64,
        t: EphemerisTime,
    ) {
        let mut world_pos = self.world.get::<&mut WorldPosition>(entity).unwrap();

        let local_offset = if let Ok(orbit) = self.world.get::<&State>(entity) {
            orbit.propagate(t, parent_mu).unwrap().r
        } else {
            vec3(0.0, 0.0, 0.0)
        };

        let new_world = parent_pos + local_offset;
        world_pos.pos = new_world;

        drop(world_pos);

        if let Some(kids) = children.get(&entity) {
            for &child in kids {
                let mu = { self.world.get::<&mut Body>(entity).unwrap().mu };
                self.propagate(children, child, new_world, mu, t);
            }
        }
    }

    // Updates craft to be on the surface of their planet
    fn landed_system(&mut self) {
        let mut pos_map = HashMap::new();
        for (entity, (world_pos, _body)) in self.world.query::<(&WorldPosition, &Body)>().iter() {
            pos_map.insert(entity, world_pos.pos);
        }

        for (_entity, (world_pos, parent, landed)) in
            self.world
                .query_mut::<(&mut WorldPosition, &Parent, &Landed)>()
        {
            let parent_pos = pos_map.get(&parent.id).unwrap();
            world_pos.pos = parent_pos + landed.offset;
        }
    }

    fn sync_models(&mut self, app: &App) {
        for (_entity, (world_pos, model, scene_obj)) in
            self.world
                .query_mut::<(&WorldPosition, &mut ModelComponent, &SceneObject)>()
        {
            let new_pos: Vec3 = nalgebra_glm::convert(world_pos.pos - self.camera_3d.world_pos);
            model.set_position(new_pos);
            self.bvh.move_obj(
                scene_obj.bvh_node_id.unwrap(),
                &app.renderer.get_model_aabb(model),
                &vec3(0.0f32, 0.0, 0.0),
            );
        }
    }

    /// Sets the selected position for the camera to orbit about based on the selected entity
    fn select_system(&mut self) {
        let Some(selected_entity) = self.selection.selected_entity() else {
            return;
        };

        // Buildings keep the camera centered on their planet, not on themselves
        let focus = if self.world.get::<&SurfaceTile>(selected_entity).is_ok() {
            self.world
                .get::<&Parent>(selected_entity)
                .map(|p| p.id)
                .unwrap_or(selected_entity)
        } else {
            selected_entity
        };

        if let Ok(world_pos) = self.world.get::<&WorldPosition>(focus) {
            self.selection.selected_pos = world_pos.pos
        }
    }

    /// Sets the selected entity based on a per-tile check on the already selected entity
    fn tile_select_system(&mut self, app: &App) {
        let pick = self.pick_hovered_tile(app);

        // If the tile is hovered, and selected, make the tile's occupant hovered and selected
        if let Some((entity, tile_index, _)) = &pick {
            let occupant = {
                let tile_map = self.world.get::<&TileMap>(*entity).unwrap();
                tile_map.occupant(*tile_index as u32)
            };

            if let Some(occupant) = occupant {
                self.hovered = Some(occupant)
            }

            if app.mouse_left_clicked && !app.is_click_consumed() {
                self.clicked_tile_key = Some((*entity, *tile_index));

                if let Some(occupant) = occupant {
                    self.selection.set_selected(occupant, app.seconds as f64);
                    app.consume_click();
                    self.gui = self.rebuild_gui(app)
                }
            }
        }

        // rebuild the line path component if the hovered tile has changed
        Self::sync_tile_path(&mut self.hovered_tile, pick, 0.45, &app.renderer);
    }

    /// Returns the selected entity, the tile index for that entity, and the tile vertices for a tile if it's
    /// hovered over, otherwise None
    fn pick_hovered_tile(&self, app: &App) -> Option<(Entity, usize, Vec<f32>)> {
        if self.hovered.is_some() {
            return None; // some other hover system already wrote to hover
        }

        let Some(selected_entity) = self.selection.selected_entity() else {
            return None; // If nothing is selected, just return
        };

        let body_entity = if self.world.get::<&Body>(selected_entity).is_ok() {
            selected_entity
        } else if let Ok(parent) = self.world.get::<&Parent>(selected_entity) {
            parent.id
        } else {
            return None;
        };

        // Check if the selected is a body
        let mut q = match self
            .world
            .query_one::<(&Body, &WorldPosition, &TileMap)>(body_entity)
        {
            Ok(q) => q,
            Err(_) => return None,
        };
        let (body, pos, tile_map) = q.get()?;

        // Exclude gaseous planets since they dont have tiles
        if body.gaseous() {
            return None;
        }

        // Check if the mouse is hovering over the planet
        let relative_pos = pos.pos - self.camera_3d.world_pos;
        let center = nalgebra_glm::convert(relative_pos);
        let body_sphere = Sphere {
            center,
            radius: body.body_radius as f32,
        };
        let mouse_ray = self.camera_3d.inner.get_ray(
            app.mouse_pos.x,
            app.mouse_pos.y,
            app.window_size.x as f32,
            app.window_size.y as f32,
        );
        let Some(_) = body_sphere.raycast(&mouse_ray) else {
            // not hovering over the planet
            return None;
        };

        // Go through each tile and figure out which one is closest to the ray intersection point
        let r = body.body_radius as f32 * 1.002;
        let local_origin = (mouse_ray.origin() - center) / r;
        let local_ray = Ray::new(local_origin, mouse_ray.dir());

        let (i, _t) = tile_map
            .tris
            .iter()
            .enumerate()
            .filter_map(|(i, tri)| tri.raycast(&local_ray).map(|t| (i, t)))
            .min_by(|a, b| a.1.total_cmp(&b.1))?;

        let vertices = self.tile_outline_vertices(body_entity, i)?;

        Some((body_entity, i, vertices))
    }

    /// Sets the hovered and selected entities for bodies and craft based on a coarse, spherical metric
    fn mouse_hover_system(&mut self, app: &App, bodies: bool) {
        if self.hovered.is_some() {
            return; // some other hover system already wrote to hover
        }

        let mouse_pos = app.mouse_pos;

        for (entity, (world_pos, _model)) in self
            .world
            .query::<hecs::Without<(&WorldPosition, &ModelComponent), &SurfaceTile>>()
            .iter()
        {
            let body = self.world.get::<&Body>(entity);
            if body.is_ok() != bodies {
                continue;
            }
            let relative_pos = world_pos.pos - self.camera_3d.world_pos;
            let screen_pos = self.world_to_screen(relative_pos, app);
            if screen_pos.is_none() {
                continue;
            }

            let radius = body.map(|b| b.body_radius).unwrap_or(0.0);

            let screen_pos = screen_pos.unwrap();
            let l1_dist = nalgebra_glm::l2_norm(&(screen_pos - mouse_pos));
            if (l1_dist as f64)
                < self
                    .apparent_radius_px(radius, relative_pos.norm(), app)
                    .max(16.0)
            {
                self.hovered = Some(entity);
                if app.mouse_left_clicked && !app.is_click_consumed() {
                    self.selection.set_selected(entity, app.seconds as f64);
                    app.consume_click();
                    self.gui = self.rebuild_gui(app);
                }
                break;
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
                    let state_now = assoc_state.propagate(self.current_et, mu).unwrap();
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

            line.color = STYLE.accent;

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

    fn get_selected_body_radius(&self) -> Option<f64> {
        let entity = self.selection.selected_entity()?;
        let mut q = self.world.query_one::<&Body>(entity).ok()?;
        let body = q.get()?;
        Some(body.body_radius)
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

        for (entity, (world_pos, _model)) in self
            .world
            .query::<hecs::Without<(&WorldPosition, &ModelComponent), &SurfaceTile>>()
            .iter()
        {
            let relative_pos = world_pos.pos - self.camera_3d.world_pos;
            if let Some(screen) = self.world_to_screen(relative_pos, app) {
                let rect = Rectangle {
                    pos: screen,
                    size: vec2(2.0, 2.0),
                };
                let radius = self
                    .world
                    .get::<&Body>(entity)
                    .map(|b| b.body_radius)
                    .unwrap_or(0.0);

                if self.apparent_radius_px(radius, relative_pos.norm(), app) < 2.0
                    && !self.is_occluded(entity, relative_pos)
                {
                    app.renderer.fill_rect(rect);
                }
            }
        }
    }

    fn is_occluded(&self, entity: Entity, relative_pos: DVec3) -> bool {
        let dist = relative_pos.norm();
        let dir = relative_pos / dist;

        for (other, (opos, obody)) in self.world.query::<(&WorldPosition, &Body)>().iter() {
            if other == entity {
                continue; // body never occludes itself
            }

            let c = opos.pos - self.camera_3d.world_pos;
            let along = c.dot(&dir);
            if along <= 0.0 || along >= dist {
                continue; // behind camera, or further away than the target
            }

            let perp_sq = c.norm_squared() - along * along;
            if perp_sq < obody.body_radius * obody.body_radius {
                return true;
            }
        }
        false
    }

    /// Radius of a body on screen in px
    fn apparent_radius_px(&self, radius: f64, dist: f64, app: &App) -> f64 {
        let ProjectionKind::Perspective { fov_rad, .. } = self.camera_3d.inner.projection_kind
        else {
            return 0.0;
        };
        (radius / dist) / (fov_rad as f64 / 2.0).tan() * (app.window_size.y as f64 / 2.0)
    }

    fn sync_tile_path(
        slot: &mut Option<(Entity, usize, LinePathComponent)>,
        want: Option<(Entity, usize, Vec<f32>)>,
        alpha: f32,
        renderer: &RenderContext,
    ) {
        let same = match (&*slot, &want) {
            (Some((e, i, _)), Some((ne, ni, _))) => e == ne && i == ni,
            (None, None) => true,
            _ => false,
        };
        if same {
            return; // same tile as last frame, keep the buffer we already have
        }

        if let Some((_, _, mut lp)) = slot.take() {
            lp.queue_deletion(renderer);
        }

        *slot = want.map(|(e, i, v)| {
            let mut line_path = LinePathComponent::new(v);
            line_path.color = vec4(1.0, 1.0, 1.0, alpha);
            line_path.width = 5.0;
            line_path.fade = false;
            (e, i, line_path)
        });
    }

    fn selected_tile_key(&self) -> Option<(Entity, usize)> {
        let sel = self.selection.selected_entity()?;

        // a selected building resolves to the tile it stands on
        if let (Ok(tile), Ok(parent)) = (
            self.world.get::<&SurfaceTile>(sel),
            self.world.get::<&Parent>(sel),
        ) {
            return Some((parent.id, tile.index as usize));
        }

        // otherwise a clicked bare tile stays lit while its body is selected
        self.clicked_tile_key.filter(|(body, _)| *body == sel)
    }

    fn sync_selected_tile(&mut self, app: &App) {
        let key = self.selected_tile_key();

        if key.is_none() {
            // clear the clicked tile
            self.clicked_tile_key = None;
        }

        let want = key.and_then(|(b, i)| self.tile_outline_vertices(b, i).map(|v| (b, i, v)));
        Self::sync_tile_path(&mut self.selected_tile, want, 1.0, &app.renderer);
    }

    fn tile_outline_vertices(&self, body: Entity, index: usize) -> Option<Vec<f32>> {
        let mut q = self.world.query_one::<(&Body, &TileMap)>(body).ok()?;
        let (b, tile_map) = q.get()?;

        let r = b.body_radius as f32 * 1.002;
        let corners: [f32; 9] = (*tile_map.tris.get(index)? * r).into();

        let mut v = Vec::with_capacity(12);
        v.extend_from_slice(&corners);
        v.extend_from_slice(&corners[0..3]);
        Some(v)
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
