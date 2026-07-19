use crate::material::{Color, MaterialDesc};
use crate::scene::Vec3;

use super::super::camera::CameraProjection;
use super::lighting::PreparedLights;
use super::materials::MaterialPass;
use super::shadows::{ShadowOccluderSet, ShadowVisibilityCache};
use super::shadows::{area_shadow_factor_profiled, directional_shadow_factor_profiled};
use super::types::{PreparedPrimitive, PrimitiveSinks, TransparentPrimitive};
use crate::render::PrepareWorkCounter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CpuBakeCorner {
    pub(super) position: Vec3,
    pub(super) geometric_normal: Vec3,
    pub(super) uv: [f32; 2],
    pub(super) tangent: Vec3,
    pub(super) tangent_handedness: f32,
    pub(super) vertex_color: Color,
    pub(super) directional_shadow_visibility: f32,
    pub(super) area_shadow_visibility: f32,
}

/// CPU texture baking never emits more than 48x48 child triangles per source
/// triangle. The adaptive footprint calculation selects a smaller factor for
/// sub-pixel texture footprints while retaining the legacy reference fidelity
/// for large, authored texture details.
pub(super) const CPU_TEXTURE_SUBDIVISION_HARD_CAP: u32 = 48;
const CPU_TEXTURE_PIXELS_PER_SUBDIVISION: f32 = 1.0;

pub(super) fn cpu_texture_subdivisions(
    material: &MaterialDesc,
    backend_shaded_material: bool,
    screen_edge_pixels: f32,
    uv_span: f32,
    texture_edge_texels: f32,
) -> u32 {
    if backend_shaded_material {
        return 1;
    }
    let transmissive = material.transmission_factor() > 0.001;
    let has_active_texture = material.base_color_texture().is_some()
        || material.normal_texture().is_some()
        || material.metallic_roughness_texture().is_some()
        || material.occlusion_texture().is_some()
        || material.emissive_texture().is_some()
        || material.clearcoat_texture().is_some()
        || material.clearcoat_roughness_texture().is_some()
        || material.clearcoat_normal_texture().is_some()
        || material.sheen_color_texture().is_some()
        || material.sheen_roughness_texture().is_some()
        || material.anisotropy_texture().is_some()
        || material.iridescence_texture().is_some()
        || material.iridescence_thickness_texture().is_some()
        || (transmissive && material.transmission_texture().is_some())
        || (transmissive
            && material.thickness_factor() > 0.0
            && material.thickness_texture().is_some());
    if !has_active_texture || screen_edge_pixels <= 0.0 || uv_span <= 0.0 {
        return 1;
    }

    let screen_footprint = screen_edge_pixels * uv_span;
    let texture_footprint = texture_edge_texels * uv_span;
    (screen_footprint.max(texture_footprint) / CPU_TEXTURE_PIXELS_PER_SUBDIVISION)
        .ceil()
        .clamp(1.0, CPU_TEXTURE_SUBDIVISION_HARD_CAP as f32) as u32
}

// Keeping the single-triangle case inline avoids a heap allocation in the CPU
// fallback's inner raster loop. The larger enum is an intentional hot-path
// tradeoff and is covered by the PF08 allocation proof.
#[allow(clippy::large_enum_variant)]
pub(super) enum SubdividedCpuCorners<'scratch> {
    Single(std::iter::Once<[CpuBakeCorner; 3]>),
    Scratch(std::iter::Copied<std::slice::Iter<'scratch, [CpuBakeCorner; 3]>>),
}

impl Iterator for SubdividedCpuCorners<'_> {
    type Item = [CpuBakeCorner; 3];

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(iter) => iter.next(),
            Self::Scratch(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Single(iter) => iter.size_hint(),
            Self::Scratch(iter) => iter.size_hint(),
        }
    }
}

impl ExactSizeIterator for SubdividedCpuCorners<'_> {}

pub(super) fn subdivided_cpu_corners<'scratch>(
    corners: [CpuBakeCorner; 3],
    subdivisions: u32,
    scratch: &'scratch mut Vec<[CpuBakeCorner; 3]>,
) -> SubdividedCpuCorners<'scratch> {
    scratch.clear();
    if subdivisions <= 1 {
        return SubdividedCpuCorners::Single(std::iter::once(corners));
    }
    scratch.reserve((subdivisions * subdivisions) as usize);
    for i in 0..subdivisions {
        for j in 0..(subdivisions - i) {
            let p00 = interpolate_cpu_corner(corners, subdivisions, i, j);
            let p10 = interpolate_cpu_corner(corners, subdivisions, i + 1, j);
            let p01 = interpolate_cpu_corner(corners, subdivisions, i, j + 1);
            scratch.push([p00, p10, p01]);
            if i + j < subdivisions - 1 {
                let p11 = interpolate_cpu_corner(corners, subdivisions, i + 1, j + 1);
                scratch.push([p10, p11, p01]);
            }
        }
    }
    SubdividedCpuCorners::Scratch(scratch.iter().copied())
}

pub(super) fn push_material_pass_primitive(
    primitive: PreparedPrimitive,
    material_pass: MaterialPass,
    sinks: &mut PrimitiveSinks<'_>,
    camera_projection: Option<&CameraProjection>,
) {
    match material_pass {
        MaterialPass::Opaque => sinks.primitives.push(primitive),
        MaterialPass::Blend => sinks.transparent_primitives.push(TransparentPrimitive {
            depth: average_sort_depth(&primitive, camera_projection),
            primitive: primitive.without_depth_prepass(),
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

#[cfg(test)]
pub(super) fn baked_shadow_visibility(
    position: Vec3,
    lights: &PreparedLights,
    shadow_occluders: &ShadowOccluderSet,
    shadow_visibility_cache: &ShadowVisibilityCache,
    backend_shaded_material: bool,
) -> f32 {
    baked_shadow_visibility_profiled(
        position,
        lights,
        shadow_occluders,
        shadow_visibility_cache,
        backend_shaded_material,
        None,
    )
}

pub(super) fn baked_shadow_visibility_profiled(
    position: Vec3,
    lights: &PreparedLights,
    shadow_occluders: &ShadowOccluderSet,
    shadow_visibility_cache: &ShadowVisibilityCache,
    backend_shaded_material: bool,
    work: Option<&PrepareWorkCounter>,
) -> f32 {
    if backend_shaded_material {
        1.0
    } else {
        shadow_visibility_cache.directional(position, work, || {
            directional_shadow_factor_profiled(position, lights, shadow_occluders, work)
        })
    }
}

pub(super) fn baked_area_shadow_visibility_profiled(
    position: Vec3,
    lights: &PreparedLights,
    shadow_occluders: &ShadowOccluderSet,
    shadow_visibility_cache: &ShadowVisibilityCache,
    work: Option<&PrepareWorkCounter>,
) -> f32 {
    if !lights.has_area_lights() {
        return 1.0;
    }
    shadow_visibility_cache.area(position, work, || {
        area_shadow_factor_profiled(position, lights, shadow_occluders, work)
    })
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
        directional_shadow_visibility: corners[0].directional_shadow_visibility * w0
            + corners[1].directional_shadow_visibility * w1
            + corners[2].directional_shadow_visibility * w2,
        area_shadow_visibility: corners[0].area_shadow_visibility * w0
            + corners[1].area_shadow_visibility * w1
            + corners[2].area_shadow_visibility * w2,
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

fn average_sort_depth(
    primitive: &PreparedPrimitive,
    camera_projection: Option<&CameraProjection>,
) -> f32 {
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

#[cfg(test)]
mod pf08_tests {
    use super::*;
    use crate::assets::TextureHandle;
    use slotmap::Key;

    #[test]
    fn pf08_texture_subdivision_is_adaptive_capped_and_transmission_gated() {
        let texture = TextureHandle::null();
        let textured = MaterialDesc::unlit(Color::WHITE).with_base_color_texture(texture);
        assert_eq!(
            cpu_texture_subdivisions(&textured, true, 4_096.0, 1.0, 256.0),
            1
        );
        assert_eq!(cpu_texture_subdivisions(&textured, false, 8.0, 1.0, 1.0), 8);
        assert_eq!(
            cpu_texture_subdivisions(&textured, false, 8.0, 1.0, 256.0),
            CPU_TEXTURE_SUBDIVISION_HARD_CAP,
            "decoded texture frequency must preserve authored details on small screen triangles"
        );
        assert_eq!(
            cpu_texture_subdivisions(&textured, false, 4_096.0, 1.0, 1.0),
            CPU_TEXTURE_SUBDIVISION_HARD_CAP
        );

        let inactive_transmission = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.2)
            .with_transmission_texture(texture);
        assert_eq!(
            cpu_texture_subdivisions(&inactive_transmission, false, 4_096.0, 1.0, 256.0),
            1,
            "a texture cannot activate transmission when the scalar factor is zero"
        );
        let active_transmission = inactive_transmission.with_transmission_factor(1.0);
        assert!(cpu_texture_subdivisions(&active_transmission, false, 256.0, 1.0, 256.0) > 1);
    }

    #[test]
    fn pf08_factor_one_uses_no_heap_and_enabled_subdivision_reuses_scratch() {
        let corners = test_corners();
        let mut scratch = Vec::new();
        let single = subdivided_cpu_corners(corners, 1, &mut scratch).collect::<Vec<_>>();
        assert_eq!(single, vec![corners]);
        assert_eq!(
            scratch.capacity(),
            0,
            "factor one must not allocate scratch"
        );

        let first = subdivided_cpu_corners(corners, 8, &mut scratch).count();
        let warm_capacity = scratch.capacity();
        let second = subdivided_cpu_corners(corners, 8, &mut scratch).count();
        assert_eq!(first, 64);
        assert_eq!(second, first);
        assert_eq!(scratch.capacity(), warm_capacity);
    }

    fn test_corners() -> [CpuBakeCorner; 3] {
        [
            corner(Vec3::ZERO, [0.0, 0.0]),
            corner(Vec3::new(1.0, 0.0, 0.0), [1.0, 0.0]),
            corner(Vec3::new(0.0, 1.0, 0.0), [0.0, 1.0]),
        ]
    }

    fn corner(position: Vec3, uv: [f32; 2]) -> CpuBakeCorner {
        CpuBakeCorner {
            position,
            geometric_normal: Vec3::new(0.0, 0.0, 1.0),
            uv,
            tangent: Vec3::new(1.0, 0.0, 0.0),
            tangent_handedness: 1.0,
            vertex_color: Color::WHITE,
            directional_shadow_visibility: 1.0,
            area_shadow_visibility: 1.0,
        }
    }
}
