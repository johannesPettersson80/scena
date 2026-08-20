use crate::assets::{Assets, MaterialHandle};
use crate::diagnostics::PrepareError;
use crate::geometry::{Aabb, GeometryDesc, GeometryTopology, GeometryVertex};
use crate::scene::{NodeKey, Scene, Transform, Vec3};

use super::super::materials::render_material_slot;
use super::super::transforms::{compose_transform, transform_position, transform_primitive};
use super::DeformationInputs;
use super::math::bounds_corners;
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
                compose_transform(node_transform, instance.transform()),
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

pub(crate) fn collect_shadow_projection_points<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
) -> Result<Vec<Vec3>, PrepareError> {
    let mut points = Vec::new();

    for (renderable, transform) in scene.renderables() {
        for primitive in renderable.primitives() {
            let primitive = transform_primitive(primitive, transform, origin_shift);
            points.extend(primitive.vertices().iter().map(|vertex| vertex.position));
        }
    }

    let Some(assets) = assets else {
        return Ok(points);
    };

    for (node, mesh, transform) in scene.mesh_nodes() {
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        append_shadow_projection_points(
            &mut points,
            node,
            &geometry,
            DeformationInputs {
                morph_weights: scene.morph_weights(node),
                skin_matrices: scene.skin_matrices(node).as_deref(),
            },
            transform,
            origin_shift,
        )?;
    }

    for (node, instance_set, node_transform) in scene.instance_set_nodes() {
        let geometry =
            assets
                .geometry(instance_set.geometry())
                .ok_or(PrepareError::GeometryNotFound {
                    node,
                    geometry: instance_set.geometry(),
                })?;
        for instance in instance_set.instances() {
            append_shadow_projection_points(
                &mut points,
                node,
                &geometry,
                DeformationInputs::default(),
                compose_transform(node_transform, instance.transform()),
                origin_shift,
            )?;
        }
    }

    Ok(points)
}

fn shadow_vertices(
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
    transform: Transform,
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

fn append_shadow_projection_points(
    points: &mut Vec<Vec3>,
    node: NodeKey,
    geometry: &GeometryDesc,
    deformation: DeformationInputs<'_>,
    transform: Transform,
    origin_shift: Vec3,
) -> Result<(), PrepareError> {
    if geometry.topology() != GeometryTopology::Triangles {
        return Ok(());
    }
    if deformation.morph_weights.is_none()
        && deformation.skin_matrices.is_none()
        && geometry.skin().is_none()
    {
        append_transformed_bounds_points(points, geometry.bounds(), transform, origin_shift);
        return Ok(());
    }

    for vertex in shadow_vertices(node, geometry, deformation)? {
        points.push(transform_position(vertex.position, transform, origin_shift));
    }
    Ok(())
}

fn append_transformed_bounds_points(
    points: &mut Vec<Vec3>,
    bounds: Aabb,
    transform: Transform,
    origin_shift: Vec3,
) {
    points.extend(
        bounds_corners(bounds)
            .into_iter()
            .map(|corner| transform_position(corner, transform, origin_shift)),
    );
}
