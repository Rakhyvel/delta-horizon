//! This module is responsible for defining the planet component
//! TODO: Should probably rename to `body.rs`, since this can represent moons and suns

use apricot::{
    bvh::{BVHNodeId, BVH},
    high_precision::WorldPosition,
    render_core::{LinePathComponent, ModelComponent, RenderContext, TextureId},
};
use hecs::{Entity, World};
use nalgebra_glm::{vec3, DVec3};

pub struct Planet {
    pub parent_planet_id: Entity,
    pub tier: u32,
    pub body_radius: f64,
    pub orbital_radius: f64,
    pub orbital_time_years: f64,
    pub rotation_period_hours: f64,
    pub rotation: f64,
    pub bvh_node_id: Option<BVHNodeId>,
    pub name: String,
    pub category: Category,
    pub atmos_pressure: f64,
    pub temperature: f64,
    pub core_mass_fraction: f64,
    pub magnetic_field: bool,
    pub density: f64,
}

/// Component relating an entity to a parent planet
pub struct Parent {
    pub parent_planet_id: Entity,
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

impl Planet {
    pub fn new(
        tier: u32,
        body_radius: f64,
        orbital_radius: f64,
        orbital_time_years: f64,
        rotation_period_hours: f64,
        core_mass_fraction: f64,
        magnetic_field: bool,
        density: f64,
        atmos_pressure: f64,
        temperature: f64,
        name: String,
        category: Category,
    ) -> Self {
        Planet {
            parent_planet_id: Entity::DANGLING,
            tier,
            body_radius,
            orbital_radius,
            orbital_time_years,
            rotation_period_hours,
            name,
            rotation: 0.0,
            bvh_node_id: None,
            category,
            atmos_pressure,
            temperature,
            core_mass_fraction,
            magnetic_field,
            density,
        }
    }

    pub fn add_as_entity(
        mut self,
        world: &mut World,
        renderer: &RenderContext,
        bvh: &mut BVH<Entity>,
    ) -> Entity {
        let planet_mesh = if self.gaseous() {
            renderer.get_mesh_id_from_name("uv").unwrap()
        } else {
            renderer.get_mesh_id_from_name("ico").unwrap()
        };

        let position: DVec3 = vec3(0., 0., 0.);
        let scale_vec: DVec3 = vec3(self.body_radius, self.body_radius, self.body_radius);

        let texture_id = self.get_texture_id(renderer);

        let planet_entity = world.spawn((
            WorldPosition { pos: position },
            ModelComponent::new(
                planet_mesh,
                texture_id,
                nalgebra_glm::convert(position),
                nalgebra_glm::convert(scale_vec),
            ),
        ));

        if self.parent_planet_id != Entity::DANGLING {
            let parent_world_pos = world
                .get::<&WorldPosition>(self.parent_planet_id)
                .unwrap()
                .pos;
            let _line_path_entity = world.spawn((
                WorldPosition {
                    pos: parent_world_pos,
                },
                Parent {
                    parent_planet_id: self.parent_planet_id,
                },
                LinePathComponent::from_orbit(self.orbital_radius as f32, 0.0, 2048),
            ));
        }

        let bvh_node_id = bvh.insert(
            planet_entity,
            renderer
                .get_mesh_aabb(planet_mesh)
                .scale(nalgebra_glm::convert(scale_vec))
                .translate(nalgebra_glm::convert(position)),
        );

        self.bvh_node_id = Some(bvh_node_id);

        world.insert(planet_entity, (self,)).unwrap();

        planet_entity
    }

    pub fn gaseous(&self) -> bool {
        self.atmos_pressure > 1.58
    }

    pub fn mass(&self) -> f64 {
        let earth_density = 5.51;
        (self.density / earth_density) * self.body_radius.powi(3)
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
