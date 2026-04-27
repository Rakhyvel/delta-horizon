use apricot::{
    bvh::BVH,
    high_precision::WorldPosition,
    render_core::{LinePathComponent, ModelComponent, RenderContext},
};
use hecs::{Entity, World};
use nalgebra_glm::{vec3, DVec3};

use crate::{
    astro::{
        escape::EscapePlan, landing::LandingPlan, launch::LaunchPlan, state::State,
        transfer::TransferPlan,
    },
    components::body::{Body, Parent, SceneObject},
};

pub struct Craft {
    pub command: Option<Command>,
    pub locked: Option<String>,
    pub line_path_entity: Option<Entity>,
    pub delta_v: f64,
}

pub struct AssociatedEntity {
    pub associate: Entity,
}

#[derive(Clone)]
pub enum Command {
    Launch { plan: LaunchPlan },
    Transfer { to: Entity, plan: TransferPlan },
    Escape { to: Entity, plan: EscapePlan },
    Land { plan: LandingPlan },
}

pub struct Landed {
    pub offset: DVec3,
}

pub fn spawn_craft(
    init_state: State,
    mut scene_obj: SceneObject,
    parent: Option<Parent>,
    world: &mut World,
    renderer: &RenderContext,
    bvh: &mut BVH<Entity>,
) -> Entity {
    let craft_mesh = renderer.get_mesh_id_from_name("cone").unwrap();

    let position: DVec3 = vec3(0., 0., 0.);
    let scale_vec: DVec3 = vec3(0.0001, 0.0001, 0.0001);

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
        let parent_world_pos = world.get::<&WorldPosition>(parent.id).unwrap().pos;
        let parent_mu = { world.get::<&Body>(parent.id).unwrap().mu };
        let line_path_entity = world.spawn((
            WorldPosition {
                pos: parent_world_pos,
            },
            parent,
            LinePathComponent::new(
                init_state
                    .generate_orbit_vertices(8192, parent_mu, None)
                    .unwrap(),
            ),
            AssociatedEntity {
                associate: craft_entity,
            },
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
                init_state,
                Craft {
                    command: None,
                    locked: None,
                    line_path_entity,
                    delta_v: 10_000.0,
                },
            ),
        )
        .unwrap();

    craft_entity
}

pub fn spawn_landed_craft(
    mut scene_obj: SceneObject,
    parent: Parent,
    world: &mut World,
    renderer: &RenderContext,
    bvh: &mut BVH<Entity>,
) -> Entity {
    let craft_mesh = renderer.get_mesh_id_from_name("cone").unwrap();

    let position: DVec3 = vec3(0., 0., 0.);
    let scale_vec: DVec3 = vec3(0.01, 0.01, 0.01);

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
                    offset: vec3(0.0, parent_radius, 0.0),
                },
                Craft {
                    command: None,
                    locked: None,
                    line_path_entity: None,
                    delta_v: 10.0,
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
    new_line_path: Option<(WorldPosition, Parent, LinePathComponent, AssociatedEntity)>,
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
