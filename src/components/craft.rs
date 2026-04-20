use apricot::{
    bvh::BVH,
    high_precision::WorldPosition,
    render_core::{LinePathComponent, ModelComponent, RenderContext},
};
use hecs::{Entity, World};
use nalgebra_glm::{vec3, DVec3};

use crate::{
    components::{
        body::{Body, Parent, SceneObject},
        orbit::{Orbit, OrbitKind},
    },
    scenes::astro::orbital_period,
};

pub struct Craft {
    pub command: Option<Command>,
    pub line_path_entity: Option<Entity>,
}

#[derive(Clone)]
pub enum Command {
    Orbit,
    Transfer { to: Entity },
    Capture,
    Land,
}

pub struct Transfer {
    pub from: Entity,
    pub to: Entity,
    pub progress: f64,
}

pub struct Landed {}

pub fn spawn_craft(
    mut orbit: Orbit,
    mut scene_obj: SceneObject,
    parent: Option<Parent>,
    world: &mut World,
    renderer: &RenderContext,
    bvh: &mut BVH<Entity>,
) -> Entity {
    let craft_mesh = renderer.get_mesh_id_from_name("cone").unwrap();

    let position: DVec3 = vec3(0., 0., 0.);
    let scale_vec: DVec3 = vec3(0.01, 0.01, 0.01);

    let texture_id = renderer.get_texture_id_from_name("europa").unwrap();

    let craft_entity = world.spawn((
        WorldPosition { pos: position },
        ModelComponent::new(
            craft_mesh,
            texture_id,
            nalgebra_glm::convert(position),
            nalgebra_glm::convert(scale_vec),
        ),
    ));

    let line_path_entity = if let Some(parent) = parent {
        {
            let parent_body = world.get::<&Body>(parent.id).unwrap();
            orbit.kind = OrbitKind::Periodic {
                period: orbital_period(orbit.semi_major_axis, parent_body.mass()),
                mean_anomaly_at_epoch: 0.0,
            };
        }
        let parent_world_pos = world.get::<&WorldPosition>(parent.id).unwrap().pos;
        let line_path_entity = world.spawn((
            WorldPosition {
                pos: parent_world_pos,
            },
            parent,
            LinePathComponent::new(orbit.generate_orbit_vertices(2048)),
        ));
        world.insert(craft_entity, (parent,)).unwrap();
        Some(line_path_entity)
    } else {
        None
    };

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
                orbit,
                Craft {
                    command: None,
                    line_path_entity,
                },
            ),
        )
        .unwrap();

    craft_entity
}

pub fn spawn_landed_craft(
    mut scene_obj: SceneObject,
    parent: Option<Parent>,
    world: &mut World,
    renderer: &RenderContext,
    bvh: &mut BVH<Entity>,
) -> Entity {
    let craft_mesh = renderer.get_mesh_id_from_name("cone").unwrap();

    let position: DVec3 = vec3(0., 0., 0.);
    let scale_vec: DVec3 = vec3(0.01, 0.01, 0.01);

    let texture_id = renderer.get_texture_id_from_name("europa").unwrap();

    let craft_entity = world.spawn((
        WorldPosition { pos: position },
        ModelComponent::new(
            craft_mesh,
            texture_id,
            nalgebra_glm::convert(position),
            nalgebra_glm::convert(scale_vec),
        ),
    ));

    if let Some(parent) = parent {
        world.insert(craft_entity, (parent,)).unwrap();
    }

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
                Landed {},
                Craft {
                    command: None,
                    line_path_entity: None,
                },
            ),
        )
        .unwrap();

    craft_entity
}

pub fn replace_line_path(
    world: &mut World,
    renderer: &RenderContext,
    craft_entity: Entity,
    new_line_path: Option<(WorldPosition, Parent, LinePathComponent)>,
) {
    let old_line_path = world.get::<&Craft>(craft_entity).unwrap().line_path_entity;

    if let Some(old) = old_line_path {
        {
            let mut line_path = world.get::<&mut LinePathComponent>(old).unwrap();
            renderer.queue_vao_deletion(&mut line_path.vao);
            renderer.queue_buffer_deletion(&mut line_path.vertices_buffer);
        }
        world.despawn(old).ok();
    }

    let new_entity = new_line_path.map(|components| world.spawn(components));
    world
        .get::<&mut Craft>(craft_entity)
        .unwrap()
        .line_path_entity = new_entity;
}
