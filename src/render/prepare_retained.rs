use std::collections::BTreeSet;

use crate::material::Color;
use crate::scene::{NodeKey, Scene, Vec3};

use super::{PreparedSceneState, prepare};
use prepare::transforms::{normal_from_model_matrix, world_from_model_matrix};

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
    for primitive in retained {
        let mut primitive = primitive.clone();
        if let Some(node) = primitive.source_node() {
            scene.node(node)?;
            if !scene.visible_for_active_camera(node) {
                continue;
            }
            let transform = scene.world_transform(node)?;
            primitive.set_world_from_model(
                world_from_model_matrix(transform, scene.origin_shift()),
                normal_from_model_matrix(transform),
            );
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
        scene.node(set.source_node())?;
        if !scene.visible_for_active_camera(set.source_node()) {
            continue;
        }
        let live = scene.instance_set(set.source_set()?)?;
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
        set.set_instances(records);
        visible_sets.push(set);
    }
    Some(visible_sets)
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
            let transform = scene.world_transform(node)?;
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
    for quad in labels.quads().iter().copied() {
        let mut quad = quad;
        if let Some(node) = quad.source_node() {
            scene.node(node)?;
            if !scene.visible_for_active_camera(node) {
                continue;
            }
            quad.set_tint(prepare::draw_uniform_tint(scene.node_tint(node).ok()?));
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
