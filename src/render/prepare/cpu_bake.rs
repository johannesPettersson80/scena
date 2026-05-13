use crate::geometry::Primitive;
use crate::material::{Color, MaterialDesc};
use crate::scene::Vec3;

use super::super::camera::CameraProjection;
use super::materials::MaterialPass;
use super::types::{PrimitiveSinks, TransparentPrimitive};

#[derive(Clone, Copy)]
pub(super) struct CpuBakeCorner {
    pub(super) position: Vec3,
    pub(super) geometric_normal: Vec3,
    pub(super) uv: [f32; 2],
    pub(super) tangent: Vec3,
    pub(super) tangent_handedness: f32,
    pub(super) vertex_color: Color,
    pub(super) shadow_visibility: f32,
}

pub(super) fn cpu_texture_subdivisions(
    material: &MaterialDesc,
    backend_shaded_material: bool,
) -> u32 {
    if backend_shaded_material {
        return 1;
    }
    if material.base_color_texture().is_some()
        || material.normal_texture().is_some()
        || material.metallic_roughness_texture().is_some()
        || material.occlusion_texture().is_some()
        || material.emissive_texture().is_some()
    {
        48
    } else {
        1
    }
}

pub(super) fn subdivided_cpu_corners(
    corners: [CpuBakeCorner; 3],
    subdivisions: u32,
) -> Vec<[CpuBakeCorner; 3]> {
    if subdivisions <= 1 {
        return vec![corners];
    }
    let mut triangles = Vec::with_capacity((subdivisions * subdivisions) as usize);
    for i in 0..subdivisions {
        for j in 0..(subdivisions - i) {
            let p00 = interpolate_cpu_corner(corners, subdivisions, i, j);
            let p10 = interpolate_cpu_corner(corners, subdivisions, i + 1, j);
            let p01 = interpolate_cpu_corner(corners, subdivisions, i, j + 1);
            triangles.push([p00, p10, p01]);
            if i + j < subdivisions - 1 {
                let p11 = interpolate_cpu_corner(corners, subdivisions, i + 1, j + 1);
                triangles.push([p10, p11, p01]);
            }
        }
    }
    triangles
}

pub(super) fn push_material_pass_primitive(
    primitive: Primitive,
    material_pass: MaterialPass,
    sinks: &mut PrimitiveSinks<'_>,
    camera_projection: Option<&CameraProjection>,
) {
    match material_pass {
        MaterialPass::Opaque => sinks.primitives.push(primitive),
        MaterialPass::Blend => sinks.transparent_primitives.push(TransparentPrimitive {
            depth: average_sort_depth(&primitive, camera_projection),
            primitive,
        }),
        MaterialPass::Mask { cutoff } => {
            if primitive
                .vertices()
                .iter()
                .any(|vertex| vertex.color.a >= cutoff)
            {
                sinks.primitives.push(primitive);
            }
        }
    }
}

fn interpolate_cpu_corner(
    corners: [CpuBakeCorner; 3],
    subdivisions: u32,
    i: u32,
    j: u32,
) -> CpuBakeCorner {
    let inv = (subdivisions as f32).recip();
    let w1 = i as f32 * inv;
    let w2 = j as f32 * inv;
    let w0 = (1.0 - w1 - w2).max(0.0);
    CpuBakeCorner {
        position: mix_vec3(
            corners[0].position,
            corners[1].position,
            corners[2].position,
            w0,
            w1,
            w2,
        ),
        geometric_normal: normalize_vec3(mix_vec3(
            corners[0].geometric_normal,
            corners[1].geometric_normal,
            corners[2].geometric_normal,
            w0,
            w1,
            w2,
        )),
        uv: [
            corners[0].uv[0] * w0 + corners[1].uv[0] * w1 + corners[2].uv[0] * w2,
            corners[0].uv[1] * w0 + corners[1].uv[1] * w1 + corners[2].uv[1] * w2,
        ],
        tangent: normalize_vec3(mix_vec3(
            corners[0].tangent,
            corners[1].tangent,
            corners[2].tangent,
            w0,
            w1,
            w2,
        )),
        tangent_handedness: if corners[0].tangent_handedness * w0
            + corners[1].tangent_handedness * w1
            + corners[2].tangent_handedness * w2
            < 0.0
        {
            -1.0
        } else {
            1.0
        },
        vertex_color: Color::from_linear_rgba(
            corners[0].vertex_color.r * w0
                + corners[1].vertex_color.r * w1
                + corners[2].vertex_color.r * w2,
            corners[0].vertex_color.g * w0
                + corners[1].vertex_color.g * w1
                + corners[2].vertex_color.g * w2,
            corners[0].vertex_color.b * w0
                + corners[1].vertex_color.b * w1
                + corners[2].vertex_color.b * w2,
            corners[0].vertex_color.a * w0
                + corners[1].vertex_color.a * w1
                + corners[2].vertex_color.a * w2,
        ),
        shadow_visibility: corners[0].shadow_visibility * w0
            + corners[1].shadow_visibility * w1
            + corners[2].shadow_visibility * w2,
    }
}

fn mix_vec3(a: Vec3, b: Vec3, c: Vec3, w0: f32, w1: f32, w2: f32) -> Vec3 {
    Vec3::new(
        a.x * w0 + b.x * w1 + c.x * w2,
        a.y * w0 + b.y * w1 + c.y * w2,
        a.z * w0 + b.z * w1 + c.z * w2,
    )
}

fn normalize_vec3(vector: Vec3) -> Vec3 {
    let length = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}

fn average_sort_depth(primitive: &Primitive, camera_projection: Option<&CameraProjection>) -> f32 {
    if let Some(camera_projection) = camera_projection {
        let vertices = primitive.vertices();
        let mut depth_sum = 0.0;
        let mut depth_count = 0;
        for vertex in vertices {
            if let Some(depth) = camera_projection.camera_depth(vertex.position) {
                depth_sum += depth;
                depth_count += 1;
            }
        }
        if depth_count > 0 {
            return depth_sum / depth_count as f32;
        }
    }

    let vertices = primitive.vertices();
    (vertices[0].position.z + vertices[1].position.z + vertices[2].position.z) / 3.0
}
