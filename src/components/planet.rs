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
    pub atmos_pressure: f32,
    pub temperature: f32,
    // felsicness: bigger = more likely felsic
    // magnetic field: bigger + spinning faster = more magnetic field
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
        body_radius: f32,
        orbital_radius: f32,
        orbital_time_years: f32,
        day_time_years: f32,
        atmos_pressure: f32,
        temperature: f32,
        name: String,
        category: Category,
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
            atmos_pressure,
            temperature,
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

        let position = nalgebra_glm::vec3(0., 0., 0.);
        let scale_vec = nalgebra_glm::vec3(self.body_radius, self.body_radius, self.body_radius);

        let texture_id = self.get_texture_id(renderer);

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
        self.atmos_pressure > 1.58
    }

    pub fn habitable(&self) -> bool {
        (0.8..1.5).contains(&self.atmos_pressure) && (270.0..300.0).contains(&self.temperature)
    }

    fn get_texture_id(&self, renderer: &RenderContext) -> TextureId {
        if self.category == Category::Star {
            return renderer.get_texture_id_from_name("sun").unwrap();
        }

        if self.gaseous() {
            if self.body_radius < 1.5 {
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
