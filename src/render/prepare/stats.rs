use crate::assets::EnvironmentDesc;
use crate::diagnostics::{Backend, Capabilities, CapabilityStatus, PrepareError};
use crate::geometry::Primitive;
use crate::scene::{Light, Scene};

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

pub(in crate::render) fn collect_depth_prepass_stats(
    primitives: &[Primitive],
    backend: Backend,
) -> PreparedDepthStats {
    let eligible_draws = depth_prepass_eligible_draws(primitives);
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

fn depth_prepass_eligible_draws(primitives: &[Primitive]) -> usize {
    primitives
        .iter()
        .filter(|primitive| primitive.depth_prepass_eligible())
        .count()
}

const fn depth_prepass_backend_supported(backend: Backend) -> bool {
    matches!(
        backend,
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2
    )
}

pub(in crate::render) fn collect_environment_prepare_stats(
    environment: Option<&EnvironmentDesc>,
) -> PreparedEnvironmentStats {
    // Report the prefilter pipeline counters for every environment that
    // produces a cubemap (bundled preview fixture) OR is declared as an
    // equirectangular HDR (where the cubemap projection happens at load
    // time when bytes are present). Both paths run the full prefilter
    // mip chain + BRDF LUT downstream.
    match environment {
        Some(environment)
            if environment.cubemap_faces().is_some() || environment.is_equirectangular_hdr() =>
        {
            PreparedEnvironmentStats {
                cubemaps: 1,
                prefilter_passes: 1,
                brdf_luts: 1,
            }
        }
        Some(_) | None => PreparedEnvironmentStats::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Primitive;

    #[test]
    fn single_primitive_scene_still_runs_depth_prepass_on_gpu_backends() {
        // Regression: BoxTextured (a single-mesh cube) used to render with
        // ghosted overdraw because depth_prepass_benefits returned false for
        // primitives.len() == 1. The color pipeline then had no depth state
        // and back faces composited through front faces via alpha blending.
        let primitives = vec![Primitive::unlit_triangle()];
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
            Primitive::unlit_triangle(),
            Primitive::unlit_triangle().without_depth_prepass(),
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
        let primitives = vec![Primitive::unlit_triangle()];

        let stats = collect_depth_prepass_stats(&primitives, Backend::Headless);

        assert_eq!(stats.passes, 0);
        assert_eq!(stats.draws, 0);
    }
}
