//! This module is responsible for defining the planet component
//! TODO: Should probably rename to `body.rs`, since this can represent moons and suns

use apricot::{
    bvh::{BVHNodeId, BVH},
    render_core::{LinePathComponent, ModelComponent, RenderContext, TextureId},
};
use hecs::{Entity, World};

pub struct Planet {
    pub parent_planet_id: Entity,
    pub tier: u32,
    pub body_radius: f32,
    pub orbital_radius: f32,
    pub orbital_time_years: f32,
    pub day_time_years: f32,
    pub rotation: f32,
    pub bvh_node_id: Option<BVHNodeId>,
    pub name: String,
    pub category: Category,
    pub zone: Zone,
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Zone {
    Hot,
    Habitable,
    Icy,
}

impl Planet {
    pub fn new(
        tier: u32,
        body_radius: f32,
        orbital_radius: f32,
        orbital_time_years: f32,
        day_time_years: f32,
        name: String,
        category: Category,
        zone: Zone,
    ) -> Self {
        Planet {
            parent_planet_id: Entity::DANGLING,
            tier,
            body_radius,
            orbital_radius,
            orbital_time_years,
            day_time_years,
            name,
            rotation: 0.0,
            bvh_node_id: None,
            category,
            zone,
        }
    }

    pub fn add_as_entity(
        mut self,
        world: &mut World,
        renderer: &RenderContext,
        bvh: &mut BVH<Entity>,
        texture_id: TextureId,
    ) -> Entity {
        let planet_mesh = if self.gaseous() {
            renderer.get_mesh_id_from_name("uv").unwrap()
        } else {
            renderer.get_mesh_id_from_name("ico").unwrap()
        };

        let position = nalgebra_glm::vec3(0., 0., 0.);
        let scale_vec = nalgebra_glm::vec3(self.body_radius, self.body_radius, self.body_radius);

        let planet_entity = world.spawn((ModelComponent::new(
            planet_mesh,
            texture_id,
            position,
            scale_vec,
        ),));

        if self.orbital_radius > 1.0 {
            world
                .insert(
                    planet_entity,
                    (LinePathComponent::from_orbit(
                        self.orbital_radius,
                        0.0,
                        1024,
                    ),),
                )
                .unwrap()
        }

        let bvh_node_id = bvh.insert(
            planet_entity,
            renderer
                .get_mesh_aabb(planet_mesh)
                .scale(scale_vec)
                .translate(position),
        );

        self.bvh_node_id = Some(bvh_node_id);

        world.insert(planet_entity, (self,)).unwrap();

        planet_entity
    }

    fn gaseous(&self) -> bool {
        match self.category {
            Category::GasGiant
            | Category::MiniNeptune
            | Category::SuperGasGiant
            | Category::Star => true,
            _ => false,
        }
    }
}
