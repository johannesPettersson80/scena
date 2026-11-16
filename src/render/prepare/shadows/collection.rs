use crate::assets::{Assets, MaterialHandle};
use crate::diagnostics::PrepareError;
use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::scene::{NodeKey, Scene, Vec3, WorldTransform};

use super::super::materials::render_material_slot;
use super::super::transforms::{transform_position, transform_primitive};
use super::DeformationInputs;
use super::{ShadowOccluder, ShadowOccluderSet};

pub(super) fn collect_occluders<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
) -> Result<ShadowOccluderSet, PrepareError> {
    let mut occluders = Vec::new();

    for (renderable, transform) in scene.renderables() {
        for primitive in renderable.primitives() {
            let primitive = transform_primitive(primitive, transform, origin_shift);
            let [a, b, c] = primitive.vertices();
            occluders.push(ShadowOccluder {
                a: a.position,
                b: b.position,
                c: c.position,
            });
        }
    }

    let Some(assets) = assets else {
        return Ok(ShadowOccluderSet::new(occluders));
    };

    for (node, mesh, transform) in scene.mesh_nodes() {
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        let vertices = shadow_vertices(
            node,
            &geometry,
            DeformationInputs {
                morph_weights: scene.morph_weights(node),
                skin_matrices: scene.skin_matrices(node).as_deref(),
            },
        )?;
        append_shadow_geometry(
            &mut occluders,
            &geometry,
            &vertices,
            transform,
            origin_shift,
        );
    }

    for (node, instance_set, node_transform) in scene.instance_set_nodes() {
        let geometry =
            assets
                .geometry(instance_set.geometry())
                .ok_or(PrepareError::GeometryNotFound {
                    node,
                    geometry: instance_set.geometry(),
                })?;
        let vertices = shadow_vertices(node, &geometry, DeformationInputs::default())?;
        for instance in instance_set.instances() {
            append_shadow_geometry(
                &mut occluders,
                &geometry,
                &vertices,
                node_transform.then(instance_set.placement(instance)),
                origin_shift,
            );
        }
    }

    Ok(ShadowOccluderSet::new(occluders))
}

pub(crate) fn cpu_shadow_visibility_required(
    scene: &Scene,
    backend_material_slots: &[MaterialHandle],
) -> bool {
    scene.mesh_nodes().any(|(_node, mesh, _transform)| {
        render_material_slot(mesh.material(), backend_material_slots) == 0
    }) || scene
        .instance_set_nodes()
        .any(|(_node, instance_set, _transform)| {
            render_material_slot(instance_set.material(), backend_material_slots) == 0
        })
}

pub(super) fn shadow_vertices(
    node: NodeKey,
    geometry: &GeometryDesc,
    deformation: DeformationInputs<'_>,
) -> Result<Vec<GeometryVertex>, PrepareError> {
    geometry
        .deformed_vertices(deformation.morph_weights, deformation.skin_matrices)
        .map(|vertices| vertices.into_owned())
        .map_err(|error| PrepareError::InvalidSkinGeometry {
            node,
            reason: format!("{error:?}"),
        })
}

fn append_shadow_geometry(
    occluders: &mut Vec<ShadowOccluder>,
    geometry: &GeometryDesc,
    vertices: &[GeometryVertex],
    transform: WorldTransform,
    origin_shift: Vec3,
) {
    if geometry.topology() != GeometryTopology::Triangles {
        return;
    }
    for triangle in geometry.indices().chunks_exact(3) {
        occluders.push(ShadowOccluder {
            a: transform_position(
                vertices[triangle[0] as usize].position,
                transform,
                origin_shift,
            ),
            b: transform_position(
                vertices[triangle[1] as usize].position,
                transform,
                origin_shift,
            ),
            c: transform_position(
                vertices[triangle[2] as usize].position,
                transform,
                origin_shift,
            ),
        });
    }
}
