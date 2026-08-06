use crate::material::{Color, MaterialDesc};
use crate::scene::Vec3;

use super::super::camera::CameraProjection;
use super::lighting::PreparedLights;
use super::materials::MaterialPass;
use super::shadows::{ShadowOccluderSet, ShadowVisibilityCache};
use super::shadows::{
    ambient_visibility_factor_profiled, area_shadow_factor_profiled,
    directional_shadow_factor_profiled,
};
use super::types::{PreparedPrimitive, PrimitiveSinks, TransparentPrimitive};
use crate::BakedAmbientOcclusionConfig;
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
    pub(super) ambient_visibility: f32,
}

/// CPU texture baking never emits more than 48x48 child triangles per source
/// triangle. The adaptive footprint calculation selects a smaller factor for
/// sub-pixel texture footprints while retaining the legacy reference fidelity
/// for large, authored texture details.
pub(super) const CPU_TEXTURE_SUBDIVISION_HARD_CAP: u32 = 48;
const CPU_TEXTURE_PIXELS_PER_SUBDIVISION: f32 = 1.0;
const AREA_SHADOW_SUBDIVISION_HARD_CAP: u32 = 32;
const AREA_SHADOW_PIXELS_PER_SUBDIVISION: f32 = 8.0;
pub(super) const GPU_SUBDIVIDED_TRIANGLES_PER_GEOMETRY: u64 = 30_000;

pub(super) fn area_shadow_subdivisions(has_area_lights: bool, screen_edge_pixels: f32) -> u32 {
    if !has_area_lights || screen_edge_pixels <= AREA_SHADOW_PIXELS_PER_SUBDIVISION {
        return 1;
    }
    (screen_edge_pixels / AREA_SHADOW_PIXELS_PER_SUBDIVISION)
        .ceil()
        .clamp(1.0, AREA_SHADOW_SUBDIVISION_HARD_CAP as f32) as u32
}

pub(super) fn area_shadow_subdivisions_for_scale(
    has_area_lights: bool,
    raster_edge_pixels: f32,
    screen_space_scale: f32,
) -> u32 {
    let logical_scale = if screen_space_scale.is_finite() {
        screen_space_scale.max(1.0)
    } else {
        1.0
    };
    area_shadow_subdivisions(has_area_lights, raster_edge_pixels / logical_scale)
}

pub(super) fn bounded_gpu_subdivisions(
    requested: u32,
    source_triangles: u32,
    gpu_backend: bool,
) -> u32 {
    let requested = requested.max(1);
    if !gpu_backend || requested == 1 {
        return requested;
    }

    let children_per_source = GPU_SUBDIVIDED_TRIANGLES_PER_GEOMETRY
        .checked_div(u64::from(source_triangles.max(1)))
        .unwrap_or(1)
        .max(1);
    requested.min(children_per_source.isqrt() as u32).max(1)
}

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
    refine_curvature: bool,
    scratch: &'scratch mut Vec<[CpuBakeCorner; 3]>,
) -> SubdividedCpuCorners<'scratch> {
    scratch.clear();
    if subdivisions <= 1 {
        return SubdividedCpuCorners::Single(std::iter::once(corners));
    }
    scratch.reserve((subdivisions * subdivisions) as usize);
    for i in 0..subdivisions {
        for j in 0..(subdivisions - i) {
            let p00 = interpolate_cpu_corner(corners, subdivisions, i, j, refine_curvature);
            let p10 = interpolate_cpu_corner(corners, subdivisions, i + 1, j, refine_curvature);
            let p01 = interpolate_cpu_corner(corners, subdivisions, i, j + 1, refine_curvature);
            scratch.push([p00, p10, p01]);
            if i + j < subdivisions - 1 {
                let p11 =
                    interpolate_cpu_corner(corners, subdivisions, i + 1, j + 1, refine_curvature);
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

pub(super) fn baked_ambient_visibility_profiled(
    position: Vec3,
    normal: Vec3,
    config: Option<BakedAmbientOcclusionConfig>,
    shadow_occluders: &ShadowOccluderSet,
    shadow_visibility_cache: &ShadowVisibilityCache,
    work: Option<&PrepareWorkCounter>,
) -> f32 {
    let Some(config) = config else {
        return 1.0;
    };
    shadow_visibility_cache.ambient(position, normal, config, work, || {
        ambient_visibility_factor_profiled(position, normal, shadow_occluders, config, work)
    })
}

fn interpolate_cpu_corner(
    corners: [CpuBakeCorner; 3],
    subdivisions: u32,
    i: u32,
    j: u32,
    refine_curvature: bool,
) -> CpuBakeCorner {
    let inv = (subdivisions as f32).recip();
    let w1 = i as f32 * inv;
    let w2 = j as f32 * inv;
    let w0 = (1.0 - w1 - w2).max(0.0);
    let linear_position = mix_vec3(
        corners[0].position,
        corners[1].position,
        corners[2].position,
        w0,
        w1,
        w2,
    );
    let position = if refine_curvature {
        phong_tessellated_position(corners, linear_position, [w0, w1, w2])
    } else {
        linear_position
    };
    CpuBakeCorner {
        position,
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
        ambient_visibility: corners[0].ambient_visibility * w0
            + corners[1].ambient_visibility * w1
            + corners[2].ambient_visibility * w2,
    }
}

fn phong_tessellated_position(
    corners: [CpuBakeCorner; 3],
    linear_position: Vec3,
    weights: [f32; 3],
) -> Vec3 {
    const CURVE_STRENGTH: f32 = 0.5;
    const MAX_LOCAL_EDGE_DISPLACEMENT_FRACTION: f32 = 0.05;
    let projected = corners
        .into_iter()
        .zip(weights)
        .fold(Vec3::ZERO, |sum, (corner, weight)| {
            let normal = normalize_vec3(corner.geometric_normal);
            let on_tangent_plane =
                linear_position - normal * (linear_position - corner.position).dot(normal);
            sum + on_tangent_plane * weight
        });
    let refined = linear_position + (projected - linear_position) * CURVE_STRENGTH;
    let local_source_edge = corners[0]
        .position
        .distance(corners[1].position)
        .min(corners[1].position.distance(corners[2].position))
        .min(corners[2].position.distance(corners[0].position));
    let displacement = refined.distance(linear_position);
    if !local_source_edge.is_finite()
        || local_source_edge <= f32::EPSILON
        || !displacement.is_finite()
        || displacement > local_source_edge * MAX_LOCAL_EDGE_DISPLACEMENT_FRACTION
        || expands_unaffected_axis(corners, refined, local_source_edge)
    {
        return linear_position;
    }
    refined
}

fn expands_unaffected_axis(
    corners: [CpuBakeCorner; 3],
    refined: Vec3,
    local_source_edge: f32,
) -> bool {
    let positions = corners.map(|corner| corner.position);
    let axis_epsilon = (local_source_edge * 1.0e-5).max(f32::EPSILON * 16.0);
    [
        ([positions[0].x, positions[1].x, positions[2].x], refined.x),
        ([positions[0].y, positions[1].y, positions[2].y], refined.y),
        ([positions[0].z, positions[1].z, positions[2].z], refined.z),
    ]
    .into_iter()
    .any(|(source, derived)| {
        let min = source.into_iter().fold(f32::INFINITY, f32::min);
        let max = source.into_iter().fold(f32::NEG_INFINITY, f32::max);
        max - min <= axis_epsilon && (derived < min - axis_epsilon || derived > max + axis_epsilon)
    })
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
        let single = subdivided_cpu_corners(corners, 1, false, &mut scratch).collect::<Vec<_>>();
        assert_eq!(single, vec![corners]);
        assert_eq!(
            scratch.capacity(),
            0,
            "factor one must not allocate scratch"
        );

        let first = subdivided_cpu_corners(corners, 8, false, &mut scratch).count();
        let warm_capacity = scratch.capacity();
        let second = subdivided_cpu_corners(corners, 8, false, &mut scratch).count();
        assert_eq!(first, 64);
        assert_eq!(second, first);
        assert_eq!(scratch.capacity(), warm_capacity);
    }

    #[test]
    fn area_shadow_receivers_subdivide_large_screen_triangles() {
        assert_eq!(area_shadow_subdivisions(false, 4_096.0), 1);
        assert_eq!(area_shadow_subdivisions(true, 8.0), 1);
        assert_eq!(area_shadow_subdivisions(true, 96.0), 12);
        assert_eq!(area_shadow_subdivisions(true, 4_096.0), 32);
    }

    #[test]
    fn ssaa_area_shadow_density_uses_logical_output_pixels() {
        assert_eq!(area_shadow_subdivisions_for_scale(true, 96.0, 1.0), 12);
        assert_eq!(area_shadow_subdivisions_for_scale(true, 96.0, 2.0), 6);
        assert_eq!(area_shadow_subdivisions_for_scale(true, 96.0, 4.0), 3);
    }

    #[test]
    fn gpu_subdivision_budget_preserves_simple_receivers_and_bounds_dense_meshes() {
        assert_eq!(bounded_gpu_subdivisions(32, 2, true), 32);
        assert_eq!(bounded_gpu_subdivisions(32, 380, true), 8);
        assert_eq!(bounded_gpu_subdivisions(32, 3_072, true), 3);
        assert_eq!(bounded_gpu_subdivisions(32, 3_072, false), 32);

        for source_triangles in [2, 188, 380, 764, 1_532, 2_208, 3_072] {
            let subdivisions = bounded_gpu_subdivisions(32, source_triangles, true);
            assert!(
                u64::from(source_triangles).saturating_mul(u64::from(subdivisions).pow(2))
                    <= GPU_SUBDIVIDED_TRIANGLES_PER_GEOMETRY
            );
        }
    }

    #[test]
    fn smooth_curved_subdivision_refines_the_rendered_silhouette() {
        let mut corners = test_corners();
        let arc = 0.1_f32;
        corners[0].position = Vec3::X;
        corners[0].geometric_normal = Vec3::X;
        corners[1].position = Vec3::new(arc.cos(), 0.0, arc.sin());
        corners[1].geometric_normal = corners[1].position;
        corners[2].position = Vec3::new(1.0, 1.0, 0.0);
        corners[2].geometric_normal = Vec3::X;
        let mut scratch = Vec::new();

        let refined = subdivided_cpu_corners(corners, 2, true, &mut scratch).collect::<Vec<_>>();
        let curved_midpoint = refined
            .iter()
            .flatten()
            .find(|corner| {
                corner.position.y.abs() < 0.000_001
                    && corner.position.x < 1.0
                    && corner.position.z > 0.01
            })
            .expect("subdivision emits the curved edge midpoint");
        let linear_midpoint = (corners[0].position + corners[1].position) * 0.5;
        let linear_radius = linear_midpoint.x.hypot(linear_midpoint.z);
        let curved_radius = curved_midpoint.position.x.hypot(curved_midpoint.position.z);

        assert!(
            curved_radius > linear_radius + 1.0e-5,
            "safe circular refinement should move the midpoint from radius \
             {linear_radius} toward the authored radius; got {curved_radius}"
        );
    }

    #[test]
    fn hero_shaft_prepare_refinement_stays_inside_the_source_edge_envelope() {
        // Exact source positions and authored normals from triangle 8 of
        // `machine:/drive_unit/drive shaft` in the frozen hero GLB. This is
        // the GLB-accessor -> prepared-position boundary that the asset-level
        // edge-rounding regression did not exercise.
        let mut corners = test_corners();
        corners[0].position = Vec3::new(0.005_694_019, -0.173_75, -0.021_250_367);
        corners[0].geometric_normal = Vec3::new(0.142_296_96, -0.835_282_2, -0.531_088_65);
        corners[1].position = Vec3::new(0.005_694_019, 0.173_75, -0.021_250_367);
        corners[1].geometric_normal = Vec3::new(0.142_296_96, 0.835_282_2, -0.531_088_65);
        corners[2].position = Vec3::new(0.007_071_668, 0.173_75, -0.020_832_462);
        corners[2].geometric_normal = Vec3::new(0.176_702_35, 0.835_311_1, -0.520_606_94);

        let linear_midpoint = (corners[0].position + corners[1].position) * 0.5;
        let prepared_midpoint =
            phong_tessellated_position(corners, linear_midpoint, [0.5, 0.5, 0.0]);
        let local_source_edge = corners[0]
            .position
            .distance(corners[1].position)
            .min(corners[1].position.distance(corners[2].position))
            .min(corners[2].position.distance(corners[0].position));
        let displacement = prepared_midpoint.distance(linear_midpoint);

        assert!(
            displacement <= local_source_edge * 0.05 + 1.0e-7,
            "hero shaft prepare-time refinement moved {displacement:.9} m, more than 5% \
             of its {local_source_edge:.9} m local source edge"
        );
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
            ambient_visibility: 1.0,
        }
    }
}
