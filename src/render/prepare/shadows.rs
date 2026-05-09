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

/// Computes the light-space view-projection matrix for a directional light.
/// Returns an orthographic `light_from_world` whose frustum tightly encloses
/// the world-space AABB of `occluders` along `light_direction`. The shader
/// projects fragment world positions through this matrix and samples the
/// shadow texture written by the shadow caster pass; closes the
/// `LightCamera` portion of scena-wgpu-architect F3 (Phase 1B).
///
/// Returned matrix is column-major 4x4 to match the WGSL `mat4x4<f32>` upload
/// layout used elsewhere in the renderer.
pub(super) fn directional_light_view_projection(
    light_direction: Vec3,
    occluders: &[ShadowOccluder],
) -> [f32; 16] {
    if occluders.is_empty() {
        return identity_matrix4();
    }

    // Light forward = direction the light travels. Build an orthonormal basis.
    let forward = normalize_or(light_direction, Vec3::new(0.0, 0.0, -1.0));
    let world_up = if forward.y.abs() > 0.99 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let right = normalize_or(cross_vec3(world_up, forward), Vec3::new(1.0, 0.0, 0.0));
    let up = cross_vec3(forward, right);

    // Project all occluder vertices into the light-view basis, then build an
    // orthographic projection that fits the resulting AABB. min/max is the
    // tight light-space bounding box.
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for occluder in occluders {
        for vertex in [occluder.a, occluder.b, occluder.c] {
            let lx = dot_vec3(right, vertex);
            let ly = dot_vec3(up, vertex);
            let lz = dot_vec3(forward, vertex);
            min = Vec3::new(min.x.min(lx), min.y.min(ly), min.z.min(lz));
            max = Vec3::new(max.x.max(lx), max.y.max(ly), max.z.max(lz));
        }
    }

    // Pad slightly so receivers near the AABB edges aren't clipped, and so
    // self-shadow acne lands inside the [near, far] window the projection
    // covers.
    let pad = ((max.x - min.x) + (max.y - min.y) + (max.z - min.z)) * 0.05;
    min = Vec3::new(min.x - pad, min.y - pad, min.z - pad);
    max = Vec3::new(max.x + pad, max.y + pad, max.z + pad);

    // Orthographic projection mapping light-view [min..max] → clip [-1..1] x
    // [-1..1] x [0..1] (WebGPU/wgpu Z range). Composed with the view rotation
    // expressing world → light-view-space.
    let inv_x = 1.0 / (max.x - min.x).max(f32::EPSILON);
    let inv_y = 1.0 / (max.y - min.y).max(f32::EPSILON);
    let inv_z = 1.0 / (max.z - min.z).max(f32::EPSILON);

    // light_from_world = ortho * view. Compose by hand so the column-major
    // upload layout matches WGSL.
    //
    // view matrix (world → light-view-space, column-major):
    //   col0 = (right.x, up.x, forward.x, 0)
    //   col1 = (right.y, up.y, forward.y, 0)
    //   col2 = (right.z, up.z, forward.z, 0)
    //   col3 = (0, 0, 0, 1)
    //
    // ortho matrix (light-view → clip, column-major, depth in [0..1]):
    //   col0 = (2 * inv_x, 0, 0, 0)
    //   col1 = (0, 2 * inv_y, 0, 0)
    //   col2 = (0, 0, inv_z, 0)
    //   col3 = (-(max.x + min.x) * inv_x,
    //           -(max.y + min.y) * inv_y,
    //           -min.z * inv_z,
    //           1)
    //
    // Combined column-major light_from_world:
    let cm = |x: f32, y: f32, z: f32, w: f32| [x, y, z, w];
    let col0 = cm(
        2.0 * right.x * inv_x,
        2.0 * up.x * inv_y,
        forward.x * inv_z,
        0.0,
    );
    let col1 = cm(
        2.0 * right.y * inv_x,
        2.0 * up.y * inv_y,
        forward.y * inv_z,
        0.0,
    );
    let col2 = cm(
        2.0 * right.z * inv_x,
        2.0 * up.z * inv_y,
        forward.z * inv_z,
        0.0,
    );
    let col3 = cm(
        -(max.x + min.x) * inv_x,
        -(max.y + min.y) * inv_y,
        -min.z * inv_z,
        1.0,
    );
    [
        col0[0], col0[1], col0[2], col0[3], col1[0], col1[1], col1[2], col1[3], col2[0], col2[1],
        col2[2], col2[3], col3[0], col3[1], col3[2], col3[3],
    ]
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length = dot_vec3(value, value).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return fallback;
    }
    Vec3::new(value.x / length, value.y / length, value.z / length)
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
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
