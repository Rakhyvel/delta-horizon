//! This module is responsible for defining the gameplay scene.

use std::{collections::HashMap, f64::consts::PI};

use apricot::{
    app::{App, Scene},
    bvh::BVH,
    camera::{Camera, ProjectionKind},
    high_precision::{self, WorldPosition},
    opengl::create_program,
    rectangle::Rectangle,
    render_core::{LinePathComponent, ModelComponent},
    shadow_map::DirectionalLightSource,
};
use hecs::{Entity, World};
use nalgebra_glm::{vec3, vec4, DVec3};
use sdl2::keyboard::Scancode;

use crate::{container, ui::text_button::TextButton};

use crate::{
    components::{
        body::{spawn_body, Body, Category, Orbit, Parent, SceneObject},
        craft::{spawn_craft, spawn_landed_craft, Craft, Landed},
    },
    generation::solar_system_gen::{self},
    ui::{
        container::{self, Container},
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

    /// Up-down view angle
    phi: f64,
    /// Side-side view angle
    theta: f64,
    /// How far the camera swivels around the currently selected body
    distance: f64,

    /// Used for tab key latch
    prev_tab_state: bool,

    gui: Container<Message>,

    turn: usize,
    turn_transition_time: f64,
}

#[derive(Clone)]
enum Message {
    NextTurn,
}

struct SelectionState {
    pub crafts: Vec<Entity>,
    pub selected: Option<usize>,

    // For swoosh animation
    pub selected_pos: DVec3,
    pub prev_selected_pos: DVec3,
    pub transition: f64,
}

impl SelectionState {
    pub fn new(crafts: Vec<Entity>) -> Self {
        Self {
            crafts,
            selected: None,
            selected_pos: vec3(0.0, 0.0, 0.0),
            prev_selected_pos: vec3(0.0, 0.0, 0.0),
            transition: 0.0,
        }
    }

    pub fn selected_entity(&self) -> Option<Entity> {
        self.selected.map(|s| self.crafts[s])
    }

    pub fn prev(&mut self, app_seconds: f64) {
        if let Some(selected) = self.selected {
            let mut new_selection = selected;
            if selected == 0 {
                new_selection = self.crafts.len() - 1;
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
            if new_selection >= self.crafts.len() {
                new_selection = 0;
            }
            self.selected = Some(new_selection);
        } else {
            self.selected = Some(0);
        }

        self.prev_selected_pos = self.selected_pos;
        self.transition = app_seconds;
    }
}

impl Scene for Gameplay {
    /// Update the scene every tick
    fn update(&mut self, app: &App) {
        // Handle all the messages from UI
        for msg in recv_msgs(app, &mut self.gui) {
            match msg {
                Message::NextTurn => {
                    if (app.seconds as f64 - self.turn_transition_time) >= 1.0 {
                        self.turn += 1;
                        self.turn_transition_time = app.seconds as f64;
                        println!("doing the next turn!")
                    }
                }
            }
        }

        self.control(app);
        self.orbit_system(app);
        self.landed_system(app);
        self.select_system();
        self.line_path_system(app);
        self.camera_update(app);
    }

    /// Render the scene to the screen when time allows
    fn render(&mut self, app: &App) {
        self.directional_light.light_dir = -self.camera_3d.inner.position().normalize();
        app.renderer.set_camera(self.camera_3d.inner);
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

        let font = app.renderer.get_font_id_from_name("font").unwrap();
        app.renderer.set_font(font);
        if let Some(selected_entity) = self.selection.selected_entity() {
            for (entity, scene_obj) in self.world.query::<&SceneObject>().iter() {
                if entity == selected_entity {
                    app.renderer
                        .draw_text(nalgebra_glm::vec2(10.0, 10.0), &scene_obj.name);
                }
            }
        }

        self.gui.render(app);

        app.renderer.draw_text(
            nalgebra_glm::vec2(
                app.window_size.x as f32 - 90.0,
                app.window_size.y as f32 - 20.0,
            ),
            format!("turn: {}", self.turn).to_string().as_str(),
        );

        app.renderer
            .render_3d_line_paths(&self.world, Some(&self.camera_3d));
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
                include_str!("../shaders/solid-color.frag"),
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
            },
            Orbit {
                semi_major_axis: 0.0,
                eccentricity: 0.0,
                inclination: 0.0,
                longitude_of_ascending_node: 0.0,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 0.0,
                period: 0.0,
            },
            SceneObject {
                bvh_node_id: None,
                name: String::from("Sun"),
            },
            None,
            &mut world,
            &app.renderer,
            &mut bvh,
        );

        let mut bodies = vec![sun_entity];
        let mut crafts = vec![];

        let mut habitable_planet = 0;
        let planets = solar_system_gen::generate();
        for system in planets {
            let planet_habitable = system.planet.0.habitable();
            let planet_entity = spawn_body(
                system.planet.0,
                system.planet.1,
                SceneObject {
                    bvh_node_id: None,
                    name: String::from("planet name"),
                },
                Some(Parent { id: sun_entity }),
                &mut world,
                &app.renderer,
                &mut bvh,
            );
            if planet_habitable {
                habitable_planet = bodies.len();
            }
            bodies.push(planet_entity);

            for moon in system.moons {
                let moon_entity = spawn_body(
                    moon.0,
                    moon.1,
                    SceneObject {
                        bvh_node_id: None,
                        name: String::from("moon name"),
                    },
                    Some(Parent { id: planet_entity }),
                    &mut world,
                    &app.renderer,
                    &mut bvh,
                );
                bodies.push(moon_entity);
            }
        }

        let craft_entity = spawn_craft(
            Orbit {
                semi_major_axis: 2.0,
                eccentricity: 0.0,
                inclination: 0.0,
                longitude_of_ascending_node: 0.0,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 0.0,
                period: 1.0 / 365.0,
            },
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
                Rectangle::new(100.0, 120.0, 90.0, 20.0,),
                "Click me!",
                vec4(0.0, 0.0, 0.0, 0.5),
                vec4(1.0, 1.0, 1.0, 0.5),
            )
            .on_click(Message::NextTurn),
        ];

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

            selection: SelectionState::new(crafts),
            phi: 2.5,
            theta: -PI / 4.0,
            distance: 20.0,
            prev_tab_state: false,

            gui,

            turn: 0,
            turn_transition_time: 0.001,
        }
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
        for (entity, _) in self.world.query::<&WorldPosition>().iter() {
            if !has_parent.contains_key(&entity) {
                roots.push(entity);
            }
        }

        const TURN_TIME: f64 = 1.0;
        let t = 1.0 / 12.0
            * (self.turn as f64
                + cubic_ease_in_out(
                    ((app.seconds as f64 - self.turn_transition_time) / TURN_TIME).min(1.0),
                ));

        // Kick off from roots
        for root in roots {
            let root_pos = vec3(0.0, 0.0, 0.0);
            self.propagate(&children, root, root_pos, t, app);
        }
    }

    fn propagate(
        &mut self,
        children: &HashMap<Entity, Vec<Entity>>,
        entity: Entity,
        parent_pos: DVec3,
        t: f64,
        app: &App,
    ) {
        // Borrow components
        let mut world_pos = self.world.get::<&mut WorldPosition>(entity).unwrap();
        let mut model = self.world.get::<&mut ModelComponent>(entity).unwrap();
        let scene_obj = self.world.get::<&SceneObject>(entity).unwrap();

        // Compute local offset if Orbit exists
        let local_offset = if let Ok(orbit) = self.world.get::<&Orbit>(entity) {
            let theta = 2.0 * PI * (t + orbit.mean_anomaly_at_epoch) / (orbit.period + 0.0001);
            vec3(
                theta.cos() * orbit.semi_major_axis,
                theta.sin() * orbit.semi_major_axis,
                0.0,
            )
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
                self.propagate(children, child, new_world, t, app);
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
            for (entity, (world_pos, _craft)) in
                self.world.query_mut::<(&mut WorldPosition, &mut Craft)>()
            {
                if entity == selected_entity {
                    self.selection.selected_pos = world_pos.pos;
                }
            }
        }
    }

    fn line_path_system(&mut self, _app: &App) {
        // Extract out the world positions
        let mut pos_map = HashMap::new();
        for (entity, world_pos) in self.world.query::<&WorldPosition>().iter() {
            pos_map.insert(entity, world_pos.pos);
        }

        // Set the origins of the line paths wrt the parent world positions
        for (entity, (line, world_pos, parent)) in
            self.world
                .query_mut::<(&mut LinePathComponent, &mut WorldPosition, &Parent)>()
        {
            let parent_pos = pos_map.get(&parent.id).unwrap();

            line.color.w = match self.selection.selected_entity() {
                Some(selected_entity) => {
                    if entity == selected_entity {
                        0.8
                    } else {
                        0.2
                    }
                }
                None => 0.2,
            };

            world_pos.pos = *parent_pos;
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
}

/// Cubic easing out function - for animation
fn cubic_ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powf(3.0) / 2.0
    }
}
