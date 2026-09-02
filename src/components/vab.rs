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
    tile::{SurfaceTile, TileMap},
};

#[allow(unused)]
pub struct Vab {}

#[allow(unused)]
pub fn spawn_vab(
    mut scene_obj: SceneObject,
    parent: Parent,
    tile_index: u32,
    world: &mut World,
    renderer: &RenderContext,
    bvh: &mut BVH<Entity>,
) -> Entity {
    let craft_mesh = renderer.get_mesh_id_from_name("cube").unwrap();

    let position: DVec3 = vec3(0., 0., 0.);
    let scale_vec: DVec3 = vec3(0.05, 0.05, 0.05);

    let texture_id = renderer.get_texture_id_from_name("europa").unwrap();

    let offset = {
        let body = world.get::<&Body>(parent.id).unwrap();
        let tile_map = world.get::<&TileMap>(parent.id).unwrap();
        tile_map.tile_offset(tile_index, body.body_radius)
    };

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
                Landed { offset },
                SurfaceTile { index: tile_index },
                Vab {},
            ),
        )
        .unwrap();

    world
        .get::<&mut TileMap>(parent.id)
        .unwrap()
        .occupy(tile_index, craft_entity);

    craft_entity
}
