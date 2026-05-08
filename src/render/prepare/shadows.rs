use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::scene::{NodeKey, Scene, Transform, Vec3};

use super::DeformationInputs;
use super::lighting::PreparedLights;
use super::transforms::{compose_transform, transform_position, transform_primitive};

#[derive(Clone, Copy)]
pub(super) struct ShadowOccluder {
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

pub(super) fn collect_shadow_occluders<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
) -> Result<Vec<ShadowOccluder>, PrepareError> {
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
        return Ok(occluders);
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

    Ok(occluders)
}

pub(super) fn directional_shadow_factor(
    position: Vec3,
    lights: &PreparedLights,
    occluders: &[ShadowOccluder],
) -> f32 {
    let Some(ray_direction) = lights.primary_shadow_ray_direction() else {
        return 1.0;
    };
    let origin = add_vec3(position, scale_vec3(ray_direction, 0.01));
    if occluders
        .iter()
        .any(|occluder| ray_intersects_triangle(origin, ray_direction, *occluder))
    {
        0.18
    } else {
        1.0
    }
}

fn shadow_vertices(
    node: NodeKey,
    geometry: &GeometryDesc,
    deformation: DeformationInputs<'_>,
) -> Result<Vec<GeometryVertex>, PrepareError> {
    let morphed_vertices = deformation
        .morph_weights
        .and_then(|weights| geometry.morphed_vertices(weights));
    let base_vertices = morphed_vertices
        .as_deref()
        .unwrap_or_else(|| geometry.vertices());
    match deformation.skin_matrices {
        Some(matrices) => geometry
            .skinned_vertices(base_vertices, matrices)
            .map(|vertices| vertices.unwrap_or_else(|| base_vertices.to_vec()))
            .map_err(|error| PrepareError::InvalidSkinGeometry {
                node,
                reason: format!("{error:?}"),
            }),
        None if geometry.skin().is_some() => Err(PrepareError::InvalidSkinGeometry {
            node,
            reason: "skinned geometry is missing a scene skin binding".to_string(),
        }),
        None => Ok(base_vertices.to_vec()),
    }
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

fn ray_intersects_triangle(origin: Vec3, direction: Vec3, triangle: ShadowOccluder) -> bool {
    let edge1 = subtract_vec3(triangle.b, triangle.a);
    let edge2 = subtract_vec3(triangle.c, triangle.a);
    let h = cross_vec3(direction, edge2);
    let determinant = dot_vec3(edge1, h);
    if determinant.abs() <= 0.000_001 {
        return false;
    }
    let inverse_determinant = determinant.recip();
    let s = subtract_vec3(origin, triangle.a);
    let u = inverse_determinant * dot_vec3(s, h);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross_vec3(s, edge1);
    let v = inverse_determinant * dot_vec3(direction, q);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = inverse_determinant * dot_vec3(edge2, q);
    t > 0.001
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}
