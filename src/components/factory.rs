use apricot::{
    bvh::BVH,
    high_precision::WorldPosition,
    render_core::{ModelComponent, RenderContext},
};
use hecs::{Entity, World};
use nalgebra_glm::{vec3, DVec3};

use crate::{
    astro::epoch::EphemerisTime,
    components::{
        body::{Body, Parent, SceneObject},
        craft::Landed,
        parts::PartRegistry,
    },
};

pub struct Factory {
    pub current_job: Option<FactoryJob>,
}

#[derive(Debug)]
pub struct FactoryJob {
    pub part_id: String,
    pub completion_et: EphemerisTime,
    pub scheduled: bool,
}

pub fn spawn_factory(
    mut scene_obj: SceneObject,
    parent: Parent,
    world: &mut World,
    renderer: &RenderContext,
    bvh: &mut BVH<Entity>,
) -> Entity {
    let craft_mesh = renderer.get_mesh_id_from_name("cube").unwrap();

    let position: DVec3 = vec3(0., 0., 0.);
    let scale_vec: DVec3 = vec3(0.1, 0.1, 0.1);

    let texture_id = renderer.get_texture_id_from_name("europa").unwrap();

    let parent_radius = { world.get::<&Body>(parent.id).unwrap().body_radius };

    let craft_entity = world.spawn((
        WorldPosition { pos: position },
        ModelComponent::new(
            craft_mesh,
            texture_id,
            nalgebra_glm::convert(position),
            nalgebra_glm::convert(scale_vec),
        ),
    ));

    let bvh_node_id = bvh.insert(
        craft_entity,
        renderer
            .get_mesh_aabb(craft_mesh)
            .scale(nalgebra_glm::convert(scale_vec))
            .translate(nalgebra_glm::convert(position)),
    );

    scene_obj.bvh_node_id = Some(bvh_node_id);

    world
        .insert(
            craft_entity,
            (
                scene_obj,
                parent,
                Landed {
                    offset: vec3(parent_radius, 0.0, 0.0),
                },
                Factory { current_job: None },
            ),
        )
        .unwrap();

    craft_entity
}

impl Factory {
    pub fn start_job(
        &mut self,
        part_id: String,
        current_et: EphemerisTime,
        registry: &PartRegistry,
    ) -> Result<(), String> {
        let part = registry.get(&part_id).ok_or("unknown part")?;
        let completion_et = current_et + EphemerisTime::from_years(part.build_time_days / 365.0);
        self.current_job = Some(FactoryJob {
            part_id,
            completion_et,
            scheduled: false,
        });
        Ok(())
    }

    pub fn is_idle(&self) -> bool {
        self.current_job.is_none()
    }
}
