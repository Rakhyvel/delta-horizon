use apricot::{
    bvh::BVH,
    high_precision::WorldPosition,
    render_core::{ModelComponent, RenderContext},
};
use hecs::{Entity, World};
use nalgebra_glm::{vec3, DVec3};

use crate::components::{
    body::{Body, Parent, SceneObject},
    craft::Landed,
};

pub struct Vab {}

pub fn spawn_vab(
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
                    offset: vec3(-parent_radius, 0.0, 0.0),
                },
                Vab {},
            ),
        )
        .unwrap();

    craft_entity
}
