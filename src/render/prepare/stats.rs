use crate::assets::EnvironmentDesc;
use crate::diagnostics::{Backend, Capabilities, CapabilityStatus, PrepareError};
use crate::scene::{Light, Scene, Vec3};

use super::{
    EnvironmentLightingProfile, PreparedPrimitive,
    lighting::{MAX_GPU_AREA_LIGHTS, PreparedLights},
};

const DIRECTIONAL_SHADOW_PCF_KERNEL: u8 = 3;
// The depth pre-pass is correctness-load-bearing, not just an optimisation:
// when it does not run, `create_unlit_pipeline` is called with
// `depth_compare: None` and the color pipeline ends up with no depth state at
// all. Triangles then composite in submission order through alpha blending,
// producing ghosted overdraw on closed meshes (back faces leaking through
// front faces). Always run the pre-pass when any primitive is eligible.
const DEPTH_PREPASS_MIN_PRIMITIVES: usize = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct PreparedLightingStats {
    pub(in crate::render) shadow_maps: u64,
    pub(in crate::render) directional_shadow_map_resolution: Option<u32>,
    pub(in crate::render) directional_shadow_pcf_kernel: Option<u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct PreparedEnvironmentStats {
    pub(in crate::render) cubemaps: u64,
    pub(in crate::render) prefilter_passes: u64,
    pub(in crate::render) brdf_luts: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct PreparedDepthStats {
    pub(in crate::render) passes: u64,
    pub(in crate::render) draws: u64,
    pub(in crate::render) reversed_z: bool,
}

pub(in crate::render) fn collect_lighting_stats(
    scene: &Scene,
    backend: Backend,
) -> Result<PreparedLightingStats, PrepareError> {
    let mut first_shadowed_directional = None;
    for (node, _light_key, light, _transform) in scene.light_nodes() {
        let Light::Directional(light) = light else {
            continue;
        };
        if !light.casts_shadows() {
            continue;
        }
        if let Some(first) = first_shadowed_directional {
            return Err(PrepareError::MultipleShadowedDirectionalLights {
                first,
                second: node,
            });
        }
        first_shadowed_directional = Some(node);
    }
    validate_gpu_light_capacity(scene, backend)?;
    Ok(if first_shadowed_directional.is_some() {
        let capabilities = Capabilities::for_backend(backend);
        PreparedLightingStats {
            shadow_maps: 1,
            directional_shadow_map_resolution: Some(
                capabilities.directional_shadow_map_default_size,
            ),
            directional_shadow_pcf_kernel: Some(DIRECTIONAL_SHADOW_PCF_KERNEL),
        }
    } else {
        PreparedLightingStats::default()
    })
}

fn validate_gpu_light_capacity(scene: &Scene, backend: Backend) -> Result<(), PrepareError> {
    if !uses_fixed_gpu_light_uniforms(backend) {
        return Ok(());
    }
    let [directional, point, spot, area] =
        PreparedLights::from_scene(scene, Vec3::ZERO).gpu_uniform_counts();
    let over_capacity = area > MAX_GPU_AREA_LIGHTS;
    if !over_capacity {
        return Ok(());
    }
    Err(PrepareError::BackendCapabilityMismatch {
        feature: "gpu_light_uniform_capacity",
        backend,
        help: format!(
            "prepared lighting has {directional} directional, {point} point, {spot} spot, and {area} area GPU light entries, but the current GPU path supports at most {MAX_GPU_AREA_LIGHTS} area entries; directional/point/spot lights beyond the fixed uniform lane require tiled light assignment"
        ),
    })
}

const fn uses_fixed_gpu_light_uniforms(backend: Backend) -> bool {
    matches!(
        backend,
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2
    )
}

#[cfg(test)]
pub(in crate::render) fn collect_depth_prepass_stats(
    primitives: &[PreparedPrimitive],
    backend: Backend,
) -> PreparedDepthStats {
    collect_depth_prepass_stats_iter(primitives.iter(), backend)
}

pub(in crate::render) fn collect_depth_prepass_stats_iter<'a>(
    primitives: impl IntoIterator<Item = &'a PreparedPrimitive>,
    backend: Backend,
) -> PreparedDepthStats {
    let eligible_draws = primitives
        .into_iter()
        .filter(|primitive| primitive.depth_prepass_eligible())
        .count();
    if eligible_draws < DEPTH_PREPASS_MIN_PRIMITIVES || !depth_prepass_backend_supported(backend) {
        PreparedDepthStats::default()
    } else {
        let capabilities = Capabilities::for_backend(backend);
        PreparedDepthStats {
            passes: 1,
            draws: eligible_draws as u64,
            reversed_z: capabilities.reversed_z_depth == CapabilityStatus::Supported,
        }
    }
}

const fn depth_prepass_backend_supported(backend: Backend) -> bool {
    matches!(
        backend,
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2
    )
}

pub(in crate::render) fn collect_environment_prepare_stats(
    environment: Option<&EnvironmentDesc>,
    backend: Backend,
) -> PreparedEnvironmentStats {
    // Report the prefilter pipeline counters for every environment that
    // produces a cubemap (bundled preview fixture) OR is declared as an
    // equirectangular HDR (where the cubemap projection happens at load
    // time when bytes are present). Both paths run the full prefilter
    // mip chain + BRDF LUT downstream.
    match environment {
        Some(environment)
            if environment.has_cubemap_face_source() || environment.is_equirectangular_hdr() =>
        {
            let sidecar_profile =
                EnvironmentLightingProfile::for_backend(backend).sidecar_profile();
            PreparedEnvironmentStats {
                cubemaps: 1,
                prefilter_passes: u64::from(
                    !environment.has_prefilter_sidecar_profile(sidecar_profile),
                ),
                brdf_luts: 1,
            }
        }
        Some(_) | None => PreparedEnvironmentStats::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{EnvironmentDesc, EnvironmentSidecarProfile};
    use crate::geometry::Primitive;
    use crate::material::Color;
    use crate::render::precompute_environment_sidecar;

    #[test]
    fn single_primitive_scene_still_runs_depth_prepass_on_gpu_backends() {
        // Regression: BoxTextured (a single-mesh cube) used to render with
        // ghosted overdraw because depth_prepass_benefits returned false for
        // primitives.len() == 1. The color pipeline then had no depth state
        // and back faces composited through front faces via alpha blending.
        let primitives = vec![prepared(Primitive::unlit_triangle())];
        for backend in [Backend::WebGl2, Backend::WebGpu, Backend::HeadlessGpu] {
            let stats = collect_depth_prepass_stats(&primitives, backend);
            assert_eq!(
                stats.passes, 1,
                "single-primitive scene must produce a depth pre-pass on \
                 {backend:?}: without it the unlit pipeline runs with no \
                 depth state and overdraws closed meshes",
            );
        }
    }

    #[test]
    fn ineligible_stroke_primitives_do_not_disable_depth_prepass_for_triangles() {
        let primitives = vec![
            prepared(Primitive::unlit_triangle()),
            prepared(Primitive::unlit_triangle().without_depth_prepass()),
        ];

        let stats = collect_depth_prepass_stats(&primitives, Backend::WebGl2);

        assert_eq!(
            stats.passes, 1,
            "depth-prepass eligible triangles must keep a WebGL2 depth pre-pass even when helper line/wire/edge primitives are present",
        );
        assert_eq!(
            stats.draws, 1,
            "depth-prepass draw count must include only eligible primitives so helper strokes are not written into the depth buffer",
        );
    }

    #[test]
    fn cpu_headless_renderer_does_not_report_gpu_depth_prepass() {
        let primitives = vec![prepared(Primitive::unlit_triangle())];

        let stats = collect_depth_prepass_stats(&primitives, Backend::Headless);

        assert_eq!(stats.passes, 0);
        assert_eq!(stats.draws, 0);
    }

    #[test]
    fn environment_prefilter_stats_count_profile_mismatched_sidecar_as_fresh_bake() {
        let path = "memory://stats-studio.hdr";
        let bytes = rle_radiance_hdr_uniform(8, 1, [64, 32, 16, 129]);
        let plain = EnvironmentDesc::from_equirectangular_hdr_bytes(path, &bytes)
            .expect("uniform HDR fixture decodes")
            .with_cubemap_resolution(8);
        let sidecar =
            precompute_environment_sidecar(&plain, EnvironmentSidecarProfile::InteractiveWebGl2)
                .expect("interactive sidecar precomputes");
        let environment =
            EnvironmentDesc::from_equirectangular_hdr_sidecar_bytes(path, &bytes, sidecar)
                .expect("sidecar env constructs")
                .expect("sidecar sha matches source")
                .with_cubemap_resolution(8);

        let webgl2_stats = collect_environment_prepare_stats(Some(&environment), Backend::WebGl2);
        let native_stats = collect_environment_prepare_stats(Some(&environment), Backend::Headless);

        assert_eq!(webgl2_stats.prefilter_passes, 0);
        assert_eq!(
            native_stats.prefilter_passes, 1,
            "native Reference prepares must report a fresh prefilter pass when only an \
             InteractiveWebGl2 sidecar is attached"
        );
    }

    fn prepared(primitive: Primitive) -> PreparedPrimitive {
        PreparedPrimitive::new(primitive, None, Color::WHITE)
    }

    fn rle_radiance_hdr_uniform(width: u32, height: u32, rgbe: [u8; 4]) -> Vec<u8> {
        assert!(width >= 8);
        assert!(width <= 127);
        let mut bytes =
            format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
        for _ in 0..height {
            bytes.push(0x02);
            bytes.push(0x02);
            bytes.push((width >> 8) as u8);
            bytes.push((width & 0xff) as u8);
            for channel in &rgbe {
                bytes.push(0x80 + width as u8);
                bytes.push(*channel);
            }
        }
        bytes
    }
}
