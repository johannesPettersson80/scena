use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::material::Color;
use crate::scene::{NodeKey, Scene, Vec3};

use super::{PreparedSceneState, prepare};
use prepare::transforms::{normal_from_model_matrix, world_from_model_matrix};

mod blend_slots;
pub(in crate::render) use blend_slots::{BlendSlotAssignment, BlendVertexSlots};
mod probe_state;
pub(in crate::render) use probe_state::PreparedProbes;
mod scene_motion;
pub(in crate::render) use scene_motion::{ContentPlacement, PreparedView};

pub(super) fn assign_original_vertex_offsets(
    primitives: Vec<prepare::PreparedPrimitive>,
) -> Vec<prepare::PreparedPrimitive> {
    let mut next_vertex_offset = 0_u32;
    primitives
        .into_iter()
        .map(|primitive| {
            let updated = primitive.with_original_vertex_offset(next_vertex_offset);
            if updated.gpu_triangle_path() {
                next_vertex_offset = next_vertex_offset.saturating_add(3);
            }
            updated
        })
        .collect()
}

pub(super) fn next_gpu_vertex_offset(primitives: &[prepare::PreparedPrimitive]) -> u32 {
    primitives
        .iter()
        .filter(|primitive| primitive.gpu_triangle_path())
        .count()
        .saturating_mul(3) as u32
}

pub(super) fn assign_original_stroke_indices(
    strokes: Vec<prepare::PreparedStrokeSegment>,
) -> Vec<prepare::PreparedStrokeSegment> {
    strokes
        .into_iter()
        .enumerate()
        .map(|(index, stroke)| stroke.with_original_segment_index(index as u32))
        .collect()
}

pub(super) fn assign_original_label_quad_indices(
    mut labels: prepare::PreparedLabelAtlas,
) -> prepare::PreparedLabelAtlas {
    for (index, quad) in labels.quads_mut().iter_mut().enumerate() {
        *quad = quad.with_original_quad_index(index as u32);
    }
    labels
}

pub(super) fn assign_original_instance_vertex_offsets(
    mut instances: Vec<prepare::PreparedInstanceSet>,
    mut next_vertex_offset: u32,
) -> Vec<prepare::PreparedInstanceSet> {
    for set in &mut instances {
        for primitive in set.primitives_mut() {
            let updated = primitive
                .clone()
                .with_original_vertex_offset(next_vertex_offset);
            *primitive = updated;
            next_vertex_offset = next_vertex_offset.saturating_add(3);
        }
    }
    instances
}

pub(super) fn filter_retained_primitives_for_scene(
    scene: &Scene,
    retained: &[prepare::PreparedPrimitive],
) -> Option<Vec<prepare::PreparedPrimitive>> {
    let mut visible = Vec::with_capacity(retained.len());
    let mut draw_transforms = BTreeMap::new();
    for primitive in retained {
        let mut primitive = primitive.clone();
        if let Some(node) = primitive.source_node() {
            scene.node(node)?;
            if !scene.visible_for_active_camera(node) {
                continue;
            }
            if primitive.source_instance().is_some() {
                // Baked under node x instance, which the node transform alone
                // cannot restore. Fall back to a full prepare.
                return None;
            }
            if primitive.world_authored() || !primitive.gpu_triangle_path() {
                // Authored in world space, or one of the stroke quads that no
                // renderer draws: the node only decides whether the triangle
                // is in the list, and it stays as a full prepare leaves it.
                visible.push(primitive);
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = draw_transforms.entry(node) {
                let transform = scene.resolved_world(node)?;
                if transform.is_mirrored() != primitive.baked_mirrored() {
                    // The retained triangles carry the winding of the old
                    // handedness; re-drawing them mirrored would cull the
                    // front faces. Fall back to a full prepare.
                    return None;
                }
                entry.insert(prepare::PreparedDrawTransform::shared(
                    world_from_model_matrix(transform, scene.origin_shift()),
                    normal_from_model_matrix(transform),
                ));
            }
            let draw_transform = draw_transforms.get(&node)?;
            if !primitive.place(Arc::clone(draw_transform)) {
                // One of the two placements has no inverse, so the uploaded
                // vertices cannot be carried between them. Fall back to a
                // full prepare.
                return None;
            }
            primitive.set_tint(prepare::draw_uniform_tint(scene.node_tint(node).ok()?));
        }
        visible.push(primitive);
    }
    Some(visible)
}

pub(super) fn filter_retained_instances_for_scene(
    scene: &Scene,
    retained: &[prepare::PreparedInstanceSet],
) -> Option<Vec<prepare::PreparedInstanceSet>> {
    let mut visible_sets = Vec::with_capacity(retained.len());
    for set in retained {
        let Some(source_set) = set.source_set() else {
            visible_sets.push(refresh_auto_batched_group(scene, set)?);
            continue;
        };
        scene.node(set.source_node())?;
        if !scene.visible_for_active_camera(set.source_node()) {
            continue;
        }
        let live = scene.instance_set(source_set)?;
        if prepare::instance_set_has_mirrored_instance(live) {
            // An instanced draw shares one winding; a mirrored instance needs
            // the per-instance bake of a full prepare.
            return None;
        }
        // The set is drawn as `node world x instance local`. The instance half
        // is rebuilt below; the node half lives in the template triangles'
        // draw transform and has to follow the node and its ancestors too.
        let node_world = scene.resolved_world(set.source_node())?;
        let node_draw_transform = prepare::PreparedDrawTransform::shared(
            world_from_model_matrix(node_world, scene.origin_shift()),
            normal_from_model_matrix(node_world),
        );
        let node_tint = scene.node_tint(set.source_node()).ok()?;
        let records = live
            .instances()
            .filter(|instance| instance.visible())
            .map(|instance| {
                prepare::PreparedInstanceRecord::new(
                    instance.id(),
                    prepare::transforms::world_from_model_matrix(instance.transform(), Vec3::ZERO),
                    prepare::transforms::normal_from_model_matrix(instance.transform()),
                    prepare::draw_uniform_tint(multiply_tint(node_tint, instance.tint())),
                )
            })
            .collect::<Vec<_>>();
        let mut set = set.clone();
        for primitive in set.primitives_mut() {
            // The template carries the winding of the handedness it was baked
            // under, and its vertices only relate to an invertible placement.
            if node_world.is_mirrored() != primitive.baked_mirrored()
                || !primitive.place(Arc::clone(&node_draw_transform))
            {
                return None;
            }
        }
        set.set_instances(records);
        visible_sets.push(set);
    }
    Some(visible_sets)
}

/// Rebuilds the records of an automatically batched group of mesh nodes from
/// where those nodes are now, leaving out members that are hidden.
///
/// `None` when a full prepare would no longer form this group: a member was
/// removed or mirrors its geometry and needs a bake of its own, the member the
/// template was baked as is hidden, or too few members remain visible.
fn refresh_auto_batched_group(
    scene: &Scene,
    group: &prepare::PreparedInstanceSet,
) -> Option<prepare::PreparedInstanceSet> {
    let mut records = Vec::with_capacity(group.member_nodes().len());
    for node in group.member_nodes() {
        let world = scene.resolved_world(*node)?;
        if !scene.visible_for_active_camera(*node) {
            if *node == group.source_node() {
                // The group is baked, clipped and attributed as its first
                // member; a full prepare would pick another one.
                return None;
            }
            continue;
        }
        if world.is_mirrored() {
            return None;
        }
        records.push(prepare::PreparedInstanceRecord::auto_batched(
            world_from_model_matrix(world, Vec3::ZERO),
            normal_from_model_matrix(world),
            prepare::draw_uniform_tint(scene.node_tint(*node).ok()?),
        ));
    }
    if records.len() < prepare::PreparedInstanceSet::MIN_AUTO_BATCHED_MEMBERS {
        return None;
    }
    let mut group = group.clone();
    group.set_instances(records);
    Some(group)
}

pub(super) fn filter_retained_strokes_for_scene(
    scene: &Scene,
    retained: &[prepare::PreparedStrokeSegment],
) -> Option<Vec<prepare::PreparedStrokeSegment>> {
    let mut visible = Vec::with_capacity(retained.len());
    for stroke in retained {
        let mut stroke = stroke.clone();
        if let Some(node) = stroke.source_node() {
            scene.node(node)?;
            if !scene.visible_for_active_camera(node) {
                continue;
            }
            if stroke.baked_per_instance() {
                // Baked under node x instance, which the node transform alone
                // cannot restore. Fall back to a full prepare.
                return None;
            }
            let transform = scene.resolved_world(node)?;
            stroke.set_world_from_model(world_from_model_matrix(transform, scene.origin_shift()));
            stroke.set_tint(prepare::draw_uniform_tint(scene.node_tint(node).ok()?));
        }
        visible.push(stroke);
    }
    Some(visible)
}

pub(super) fn filter_retained_labels_for_scene(
    scene: &Scene,
    retained: &prepare::PreparedLabelAtlas,
) -> Option<prepare::PreparedLabelAtlas> {
    let mut labels = retained.clone();
    let mut visible = Vec::with_capacity(labels.quads().len());
    // A full prepare lays labels out untinted, so only visibility is
    // re-derived here.
    for quad in labels.quads().iter().copied() {
        if let Some(node) = quad.source_node() {
            scene.node(node)?;
            if !scene.visible_for_active_camera(node) {
                continue;
            }
        }
        visible.push(quad);
    }
    labels.set_quads(visible);
    Some(labels)
}

pub(super) fn retained_template_covers_visible_sources(
    scene: &Scene,
    retained: &[prepare::PreparedPrimitive],
    retained_strokes: &[prepare::PreparedStrokeSegment],
    retained_labels: &prepare::PreparedLabelAtlas,
    retained_instances: &[prepare::PreparedInstanceSet],
) -> bool {
    let mut retained_sources = retained
        .iter()
        .filter_map(prepare::PreparedPrimitive::source_node)
        .collect::<BTreeSet<NodeKey>>();
    retained_sources.extend(
        retained_strokes
            .iter()
            .filter_map(prepare::PreparedStrokeSegment::source_node),
    );
    retained_sources.extend(
        retained_labels
            .quads()
            .iter()
            .filter_map(prepare::PreparedLabelQuad::source_node),
    );
    retained_sources.extend(
        retained_instances
            .iter()
            .map(prepare::PreparedInstanceSet::source_node),
    );
    retained_sources.extend(
        retained_instances
            .iter()
            .flat_map(|set| set.member_nodes().iter().copied()),
    );

    scene
        .renderable_nodes()
        .all(|(node, _, _)| retained_sources.contains(&node))
        && scene
            .mesh_nodes()
            .all(|(node, _, _)| retained_sources.contains(&node))
        && scene
            .instance_set_nodes()
            .all(|(node, _, _)| retained_sources.contains(&node))
        && scene
            .label_nodes()
            .all(|(node, _, _, _)| retained_sources.contains(&node))
        && scene
            .particle_set_nodes()
            .all(|(node, _, _)| retained_sources.contains(&node))
}

pub(super) fn prepared_instance_count(prepared: &PreparedSceneState) -> u64 {
    prepared
        .instances
        .iter()
        .map(|set| set.instances().len() as u64)
        .sum()
}

fn multiply_tint(node_tint: Option<Color>, instance_tint: Option<Color>) -> Option<Color> {
    match (node_tint, instance_tint) {
        (Some(left), Some(right)) => Some(Color::from_linear_rgba(
            left.r * right.r,
            left.g * right.g,
            left.b * right.b,
            left.a * right.a,
        )),
        (Some(tint), None) | (None, Some(tint)) => Some(tint),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::assets::Assets;
    use crate::geometry::{GeometryDesc, Primitive};
    use crate::material::{Color, MaterialDesc};
    use crate::scene::{Scene, Transform};

    use super::*;

    #[test]
    fn gpu_vertex_offsets_skip_non_gpu_retained_primitives_before_instances() {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.1, 0.1, 0.1));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        let (node, set) = scene
            .add_instance_set_node(scene.root(), geometry, material, Transform::IDENTITY)
            .expect("instance set inserts");
        let instance = scene
            .push_instance(set, Transform::IDENTITY)
            .expect("instance inserts");

        let retained_primitives = assign_original_vertex_offsets(vec![
            prepared_triangle(true),
            prepared_triangle(false),
            prepared_triangle(true),
        ]);
        assert_eq!(retained_primitives[0].original_vertex_offset(), 0);
        assert_eq!(
            retained_primitives[1].original_vertex_offset(),
            3,
            "non-GPU retained primitives may share the next GPU offset because they are never encoded into the triangle vertex buffer",
        );
        assert_eq!(
            retained_primitives[2].original_vertex_offset(),
            3,
            "later GPU primitives must stay contiguous in the encoded GPU vertex buffer"
        );

        let instances = assign_original_instance_vertex_offsets(
            vec![prepare::PreparedInstanceSet::new(
                node,
                set,
                vec![prepared_triangle(true)],
                vec![prepare::PreparedInstanceRecord::new(
                    instance,
                    identity_matrix4(),
                    identity_matrix4(),
                    Color::WHITE,
                )],
            )],
            next_gpu_vertex_offset(&retained_primitives),
        );

        assert_eq!(
            instances[0].primitives()[0].original_vertex_offset(),
            6,
            "instance templates start immediately after encoded GPU triangle vertices, not after CPU-only retained primitives"
        );
    }

    fn prepared_triangle(gpu_triangle_path: bool) -> prepare::PreparedPrimitive {
        let primitive =
            prepare::PreparedPrimitive::new(Primitive::unlit_triangle(), None, Color::WHITE);
        if gpu_triangle_path {
            primitive
        } else {
            primitive.without_gpu_triangle_path()
        }
    }

    const fn identity_matrix4() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }
}
