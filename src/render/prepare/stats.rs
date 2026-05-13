use crate::assets::EnvironmentDesc;
use crate::diagnostics::{Backend, Capabilities, CapabilityStatus, PrepareError};
use crate::geometry::Primitive;
use crate::scene::{Light, Scene};

const DIRECTIONAL_SHADOW_PCF_KERNEL: u8 = 3;
const DEPTH_PREPASS_MIN_PRIMITIVES: usize = 2;

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
    if !depth_prepass_benefits(primitives) {
        PreparedDepthStats::default()
    } else {
        let capabilities = Capabilities::for_backend(backend);
        PreparedDepthStats {
            passes: 1,
            draws: primitives.len() as u64,
            reversed_z: capabilities.reversed_z_depth == CapabilityStatus::Supported,
        }
    }
}

fn depth_prepass_benefits(primitives: &[Primitive]) -> bool {
    primitives.len() >= DEPTH_PREPASS_MIN_PRIMITIVES
        && primitives.iter().all(Primitive::depth_prepass_eligible)
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
