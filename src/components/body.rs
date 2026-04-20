//! This module is responsible for defining the body component

use apricot::{
    bvh::{BVHNodeId, BVH},
    high_precision::WorldPosition,
    render_core::{LinePathComponent, ModelComponent, RenderContext, TextureId},
};
use hecs::{Entity, World};
use nalgebra_glm::{vec3, DVec3};

use crate::components::orbit::Orbit;

pub struct SceneObject {
    pub bvh_node_id: Option<BVHNodeId>,
    pub name: String,
}

pub struct Body {
    pub category: Category,
    pub body_radius: f64, // In earth radii
    pub rotation_period_hours: f64,
    pub rotation: f64,
    pub atmos_pressure: f64, // In bar
    pub temperature: f64,    // In K
    pub core_mass_fraction: f64,
    pub magnetic_field: bool,
    pub density: f64, // In g/cm^3
}

/// Component relating an entity to a parent body
#[derive(Clone, Copy)]
pub struct Parent {
    pub id: Entity,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Category {
    Dwarf,
    SubEarth,
    EarthLike,
    SuperEarth,
    MiniNeptune,
    GasGiant,
    SuperGasGiant,
    Star,
}

pub fn spawn_body(
    body: Body,
    orbit: Orbit,
    mut scene_obj: SceneObject,
    parent: Option<Parent>,
    world: &mut World,
    renderer: &RenderContext,
    bvh: &mut BVH<Entity>,
) -> Entity {
    let body_mesh = if body.gaseous() {
        renderer.get_mesh_id_from_name("uv").unwrap()
    } else {
        renderer.get_mesh_id_from_name("ico").unwrap()
    };

    let position: DVec3 = vec3(0., 0., 0.);
    let scale_vec: DVec3 = vec3(body.body_radius, body.body_radius, body.body_radius);

    let texture_id = body.get_texture_id(renderer);

    let body_entity = world.spawn((
        WorldPosition { pos: position },
        ModelComponent::new(
            body_mesh,
            texture_id,
            nalgebra_glm::convert(position),
            nalgebra_glm::convert(scale_vec),
        ),
    ));

    if let Some(parent) = parent {
        let parent_world_pos = world.get::<&WorldPosition>(parent.id).unwrap().pos;
        let _line_path_entity = world.spawn((
            WorldPosition {
                pos: parent_world_pos,
            },
            parent,
            LinePathComponent::new(orbit.generate_orbit_vertices(2048, None)),
        ));
        world.insert(body_entity, (parent,)).unwrap();
    }

    let bvh_node_id = bvh.insert(
        body_entity,
        renderer
            .get_mesh_aabb(body_mesh)
            .scale(nalgebra_glm::convert(scale_vec))
            .translate(nalgebra_glm::convert(position)),
    );

    scene_obj.bvh_node_id = Some(bvh_node_id);

    world.insert(body_entity, (scene_obj, orbit, body)).unwrap();

    body_entity
}

impl Body {
    pub fn gaseous(&self) -> bool {
        self.atmos_pressure > 1.58
    }

    pub fn mass(&self) -> f64 {
        let earth_density = 5.51;
        (self.density / earth_density) * self.body_radius.powi(3) / earth_density
    }

    pub fn habitable(&self) -> bool {
        (0.8..1.5).contains(&self.atmos_pressure) && (270.0..300.0).contains(&self.temperature)
    }

    pub fn is_giant(&self) -> bool {
        self.body_radius > 2.5
    }

    fn get_texture_id(&self, renderer: &RenderContext) -> TextureId {
        if self.category == Category::Star {
            return renderer.get_texture_id_from_name("sun").unwrap();
        }

        if self.gaseous() {
            if !self.is_giant() {
                renderer.get_texture_id_from_name("venus").unwrap()
            } else if self.temperature > 120.0 {
                renderer.get_texture_id_from_name("jupiter").unwrap()
            } else {
                renderer.get_texture_id_from_name("uranus").unwrap()
            }
        } else {
            if self.habitable() {
                renderer.get_texture_id_from_name("earth").unwrap()
            } else if self.temperature < 200.0 {
                renderer.get_texture_id_from_name("europa").unwrap()
            } else {
                renderer.get_texture_id_from_name("moon").unwrap()
            }
        }
    }
}
