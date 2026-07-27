use std::sync::Arc;

use crate::assets::{EnvironmentDesc, EnvironmentPrefilterSidecar, EnvironmentSidecarProfile};
use crate::diagnostics::AssetError;
use crate::diagnostics::Backend;
use crate::scene::Vec3;

use super::super::pbr_brdf;
use super::environment_baker::{
    EnvironmentBakeMetrics, EnvironmentIblBakeQuality, EnvironmentIblBakeRequest,
    bake_environment_ibl, bake_environment_ibl_profiled, prefilter_lod_for_roughness,
    sample_prefiltered_cubemap_lod,
};
use super::pbr_contract::{PbrMaterial, environment_split_sum_contribution, reflect_vec3};

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn environment_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_environment_step(label: &str, start_ms: f64) -> f64 {
    let now = environment_now_ms();
    if crate::diagnostics::browser_timing_enabled() {
        web_sys::console::log_1(
            &format!("[scena-demo] environment {label}: {:.1}ms", now - start_ms).into(),
        );
    }
    now
}

fn warn_environment_sidecar_profile_mismatch(
    environment: &EnvironmentDesc,
    requested: EnvironmentSidecarProfile,
    actual: EnvironmentSidecarProfile,
) {
    let message = format!(
        "scena environment warning: sidecar '{}' has profile {}, but this backend requested {}; \
         ignoring the sidecar and baking IBL from the HDR source instead",
        environment.source_path().as_str(),
        actual.name(),
        requested.name()
    );
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&message));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("{message}");
    }
}

/// Number of GGX-prefiltered specular mip levels emitted for the
/// environment cubemap. Mip 0 carries the source radiance; mips 1+
/// integrate the GGX kernel at roughness values from the shared
/// low-roughness-concentrated prefilter mapping.
pub(in crate::render) const PREFILTER_MIP_COUNT: u32 = 5;
/// 2D BRDF LUT resolution. The split-sum approximation indexes the LUT
/// by `(N·V, roughness)`; 64×64 is enough resolution for visually
/// smooth specular without blowing the GPU upload budget.
pub(in crate::render) const BRDF_LUT_SIZE: u32 = 64;
const HDR_DIFFUSE_IBL_RESPONSE_SCALE: f32 = 1.0;
const HDR_IBL_INTENSITY_SCALE: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::render) enum EnvironmentLightingProfile {
    Reference,
    InteractiveWebGl2,
}

impl EnvironmentLightingProfile {
    pub(in crate::render) fn for_backend(backend: Backend) -> Self {
        match backend {
            Backend::WebGl2 => Self::InteractiveWebGl2,
            Backend::Headless
            | Backend::HeadlessGpu
            | Backend::SurfaceDescriptor
            | Backend::NativeSurface
            | Backend::WebGpu => Self::Reference,
        }
    }

    fn prefilter_quality(self) -> EnvironmentIblBakeQuality {
        match self {
            Self::Reference => EnvironmentIblBakeQuality::Reference,
            Self::InteractiveWebGl2 => EnvironmentIblBakeQuality::InteractiveWebGl2,
        }
    }

    fn brdf_lut_size(self) -> u32 {
        BRDF_LUT_SIZE
    }

    fn brdf_sample_count(self) -> u32 {
        match self {
            Self::Reference => 1024,
            Self::InteractiveWebGl2 => 64,
        }
    }

    pub(crate) const fn sidecar_profile(self) -> EnvironmentSidecarProfile {
        match self {
            Self::Reference => EnvironmentSidecarProfile::Reference,
            Self::InteractiveWebGl2 => EnvironmentSidecarProfile::InteractiveWebGl2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedEnvironmentLighting {
    diffuse_rgb: Vec3,
    specular_rgb: Vec3,
    intensity: f32,
    rotation_y_radians: f32,
    /// Phase 1C step 1: real cubemap radiance, decoded at prepare time from
    /// the active environment asset's six face-radiance values. The `Arc`
    /// keeps `PreparedEnvironmentLighting::clone` allocation-free in the hot
    /// CPU shading loops while still letting the GPU upload consume the same
    /// pixel data without copying. The pipeline keeps a 1×1 placeholder bind
    /// when this is `None` so the GPU bind group is always well-formed.
    cubemap: Option<Arc<PreparedEnvironmentCubemap>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedEnvironmentCubemap {
    pub(in crate::render) resolution: u32,
    /// Phase 1C step 2: full GGX-prefiltered specular mip chain
    /// (PREFILTER_MIP_COUNT levels). Mip 0 is the source radiance, mips
    /// 1+ are convolved with a GGX kernel at increasing roughness. Each
    /// element is six face buffers laid out RGBA32F at that mip's
    /// resolution. The CPU rasterizer reads `mips[0]` as a six-face
    /// cube; the GPU upload streams every mip per face into the
    /// `texture_cube<f32>` mip chain.
    pub(in crate::render) mips: Vec<[Vec<f32>; 6]>,
    pub(in crate::render) mip_count: u32,
    /// 2D BRDF LUT (BRDF_LUT_SIZE × BRDF_LUT_SIZE) of `(scale, bias)`
    /// pairs that drive the split-sum specular composition
    /// `prefiltered * (F0 * scale + bias)` in the WGSL fragment shader.
    pub(in crate::render) brdf_lut: Vec<f32>,
    pub(in crate::render) brdf_lut_size: u32,
}

// Visibility note: both PreparedEnvironmentLighting and
// PreparedEnvironmentCubemap declare `pub(in crate::render)` to allow the
// GPU upload path in `crate::render::gpu` to consume the prepared cubemap
// while keeping these types out of the public crate surface.

impl Default for PreparedEnvironmentLighting {
    fn default() -> Self {
        Self {
            diffuse_rgb: Vec3::ZERO,
            specular_rgb: Vec3::ZERO,
            intensity: 0.0,
            rotation_y_radians: 0.0,
            cubemap: None,
        }
    }
}

impl PreparedEnvironmentLighting {
    pub(in crate::render) fn from_environment_with_profile(
        environment: Option<&EnvironmentDesc>,
        profile: EnvironmentLightingProfile,
    ) -> Self {
        let Some(environment) = environment else {
            return Self::default();
        };
        // Phase 1C step 1: parse the cubemap regardless of whether the CPU
        // shading path is going to consume scalar irradiance, so the GPU
        // pipeline can sample real per-fragment radiance. The scalar
        // diffuse/specular still come from `preview_irradiance_rgb` to keep
        // CPU rasterizer parity with the pre-Phase-1C fixtures.
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let environment_total_start = environment_now_ms();
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let mut environment_step_start = environment_total_start;

        let sidecar_profile = profile.sidecar_profile();
        let sidecar = environment.prefilter_sidecar(sidecar_profile);
        if sidecar.is_none()
            && let Some(actual_profile) = environment.prefilter_sidecar_profile()
        {
            warn_environment_sidecar_profile_mismatch(environment, sidecar_profile, actual_profile);
        }
        let cubemap_faces = if sidecar.is_some() {
            None
        } else {
            environment.cubemap_faces()
        };
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            environment_step_start = log_environment_step("cubemap_faces", environment_step_start);
        }
        let cubemap = if let Some(sidecar) = sidecar {
            #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
            {
                log_environment_step("load_prefilter_sidecar", environment_step_start);
            }
            Some(Arc::new(PreparedEnvironmentCubemap {
                resolution: sidecar.cubemap_resolution(),
                mips: sidecar.mips().to_vec(),
                mip_count: sidecar.mip_count(),
                brdf_lut: sidecar.brdf_lut().to_vec(),
                brdf_lut_size: sidecar.brdf_lut_size(),
            }))
        } else {
            cubemap_faces.map(|faces| {
                let resolution = faces.resolution();
                let source_pixels = faces.build_face_pixels_rgba32f();
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                let bake_start =
                    log_environment_step("build_face_pixels_rgba32f", environment_step_start);
                let baked = bake_environment_ibl(
                    &source_pixels,
                    EnvironmentIblBakeRequest {
                        source_resolution: resolution,
                        mip_count: PREFILTER_MIP_COUNT,
                        quality: profile.prefilter_quality(),
                        brdf_lut_size: profile.brdf_lut_size(),
                        brdf_sample_count: profile.brdf_sample_count(),
                    },
                );
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                {
                    log_environment_step("bake_environment_ibl", bake_start);
                }
                Arc::new(PreparedEnvironmentCubemap {
                    resolution,
                    mips: baked.mips,
                    mip_count: baked.mip_count,
                    brdf_lut: baked.brdf_lut,
                    brdf_lut_size: baked.brdf_lut_size,
                })
            })
        };
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            log_environment_step("from_environment total", environment_total_start);
        }
        // glTF/PBR color-contract fallback: when the environment records no scalar
        // `preview_irradiance_rgb` but does carry a real cubemap (the common
        // case for bundled HDR environments), derive an average radiance from
        // the cubemap mip-0 pixels so the CPU rasterizer's PBR path can still
        // light metallic surfaces. This is a generic environment fallback, not
        // an asset-specific color calibration path.
        let irradiance = match environment.preview_irradiance_rgb() {
            Some(stored) => stored,
            None => match cubemap.as_ref() {
                Some(prepared) => average_cubemap_radiance(prepared),
                None => {
                    return Self {
                        diffuse_rgb: Vec3::ZERO,
                        specular_rgb: Vec3::ZERO,
                        intensity: 0.0,
                        rotation_y_radians: 0.0,
                        cubemap,
                    };
                }
            },
        };
        let diffuse_scale = if environment.is_equirectangular_hdr() {
            HDR_DIFFUSE_IBL_RESPONSE_SCALE
        } else {
            1.0
        };
        let diffuse_rgb = Vec3::new(
            sanitize_environment_channel(irradiance[0]),
            sanitize_environment_channel(irradiance[1]),
            sanitize_environment_channel(irradiance[2]),
        ) * diffuse_scale;
        if diffuse_rgb.x <= f32::EPSILON
            && diffuse_rgb.y <= f32::EPSILON
            && diffuse_rgb.z <= f32::EPSILON
        {
            return Self {
                diffuse_rgb: Vec3::ZERO,
                specular_rgb: Vec3::ZERO,
                intensity: 0.0,
                rotation_y_radians: 0.0,
                cubemap,
            };
        }
        let intensity = if environment.is_equirectangular_hdr() {
            HDR_IBL_INTENSITY_SCALE
        } else {
            1.0
        };
        let specular_rgb = if cubemap.is_some() {
            Vec3::new(1.0, 1.0, 1.0)
        } else {
            Vec3::new(
                sanitize_environment_channel(irradiance[0]),
                sanitize_environment_channel(irradiance[1]),
                sanitize_environment_channel(irradiance[2]),
            )
        };
        Self {
            diffuse_rgb,
            specular_rgb,
            intensity,
            rotation_y_radians: 0.0,
            cubemap,
        }
    }

    pub(in crate::render) fn with_controls(
        mut self,
        intensity_scale: f32,
        rotation_y_radians: f32,
    ) -> Self {
        self.intensity *= intensity_scale.clamp(0.0, 16.0);
        self.rotation_y_radians = rotation_y_radians;
        self
    }

    pub(in crate::render) fn cubemap(&self) -> Option<&PreparedEnvironmentCubemap> {
        self.cubemap.as_deref()
    }

    pub(in crate::render::prepare) fn is_active(&self) -> bool {
        self.intensity > 0.0
            && (self.diffuse_rgb.x > f32::EPSILON
                || self.diffuse_rgb.y > f32::EPSILON
                || self.diffuse_rgb.z > f32::EPSILON
                || self.specular_rgb.x > f32::EPSILON
                || self.specular_rgb.y > f32::EPSILON
                || self.specular_rgb.z > f32::EPSILON)
    }

    pub(in crate::render::prepare) fn gpu_diffuse_intensity(&self) -> [f32; 4] {
        [
            self.diffuse_rgb.x,
            self.diffuse_rgb.y,
            self.diffuse_rgb.z,
            self.intensity,
        ]
    }

    pub(in crate::render::prepare) fn gpu_specular_intensity(&self) -> [f32; 4] {
        [
            self.specular_rgb.x,
            self.specular_rgb.y,
            self.specular_rgb.z,
            self.intensity,
        ]
    }

    pub(in crate::render::prepare) fn gpu_environment_transform(&self) -> [f32; 4] {
        [
            self.rotation_y_radians.cos(),
            self.rotation_y_radians.sin(),
            0.0,
            0.0,
        ]
    }

    pub(in crate::render::prepare) fn pbr_contribution(
        &self,
        material: PbrMaterial,
        normal: Vec3,
        view: Vec3,
    ) -> Vec3 {
        if !self.is_active() {
            return Vec3::ZERO;
        }
        let diffuse = self.diffuse_rgb;
        let reflection = rotate_environment_y(
            reflect_vec3(Vec3::new(-view.x, -view.y, -view.z), normal),
            self.rotation_y_radians,
        );
        let prefiltered = self
            .cubemap
            .as_deref()
            .map(|cubemap| sample_prefiltered_specular(cubemap, reflection, material.roughness))
            .unwrap_or(self.specular_rgb);
        let brdf = if self.cubemap.is_some() {
            pbr_brdf::split_sum_brdf_approx(dot_vec3(normal, view), material.roughness)
        } else {
            (1.0, 0.0)
        };
        scale_vec3(
            environment_split_sum_contribution(material, normal, view, diffuse, prefiltered, brdf),
            self.intensity,
        )
    }
}

fn rotate_environment_y(direction: Vec3, radians: f32) -> Vec3 {
    let (sin, cos) = radians.sin_cos();
    Vec3::new(
        cos * direction.x + sin * direction.z,
        direction.y,
        -sin * direction.x + cos * direction.z,
    )
}

#[doc(hidden)]
pub fn precompute_environment_sidecar(
    environment: &EnvironmentDesc,
    profile: EnvironmentSidecarProfile,
) -> Result<EnvironmentPrefilterSidecar, AssetError> {
    precompute_environment_sidecar_profiled(environment, profile).map(|(sidecar, _metrics)| sidecar)
}

/// Precomputes an environment sidecar and returns deterministic bake-work counters.
#[doc(hidden)]
pub fn precompute_environment_sidecar_profiled(
    environment: &EnvironmentDesc,
    profile: EnvironmentSidecarProfile,
) -> Result<(EnvironmentPrefilterSidecar, EnvironmentBakeMetrics), AssetError> {
    let render_profile = match profile {
        EnvironmentSidecarProfile::InteractiveWebGl2 => {
            EnvironmentLightingProfile::InteractiveWebGl2
        }
        EnvironmentSidecarProfile::Reference => EnvironmentLightingProfile::Reference,
    };
    let source_sha = environment
        .source_sha256()
        .ok_or_else(|| AssetError::Parse {
            path: environment.source_path().as_str().to_string(),
            reason:
                "environment sidecar generation requires source SHA-256; load the HDR from bytes"
                    .to_string(),
        })?;
    let faces = environment
        .cubemap_faces()
        .ok_or_else(|| AssetError::Parse {
            path: environment.source_path().as_str().to_string(),
            reason: "environment sidecar generation requires decoded cubemap faces".to_string(),
        })?;
    let resolution = faces.resolution();
    let source_pixels = faces.build_face_pixels_rgba32f();
    let (baked, metrics) = bake_environment_ibl_profiled(
        &source_pixels,
        EnvironmentIblBakeRequest {
            source_resolution: resolution,
            mip_count: PREFILTER_MIP_COUNT,
            quality: render_profile.prefilter_quality(),
            brdf_lut_size: render_profile.brdf_lut_size(),
            brdf_sample_count: render_profile.brdf_sample_count(),
        },
    );
    let diffuse_rgb = environment
        .preview_irradiance_rgb()
        .unwrap_or_else(|| faces.lambertian_irradiance());
    let sidecar = EnvironmentPrefilterSidecar::new(
        profile,
        source_sha,
        resolution,
        baked.mips,
        baked.brdf_lut,
        baked.brdf_lut_size,
        diffuse_rgb,
    )?;
    Ok((sidecar, metrics))
}

pub(in crate::render) fn collect_environment_lighting(
    environment: Option<&EnvironmentDesc>,
    backend: Backend,
) -> PreparedEnvironmentLighting {
    PreparedEnvironmentLighting::from_environment_with_profile(
        environment,
        EnvironmentLightingProfile::for_backend(backend),
    )
}

/// Average mip-0 radiance across the six cubemap faces. Used as a fallback
/// scalar irradiance for the CPU rasterizer when the asset does not record a
/// pre-baked `preview_irradiance_rgb` value. Without this, metallic surfaces
/// (where `1 − metallic = 0` cancels the diffuse term) get zero light from
/// the environment on the CPU path and render as pitch-black silhouettes.
fn average_cubemap_radiance(cubemap: &PreparedEnvironmentCubemap) -> [f32; 3] {
    let Some(faces) = cubemap.mips.first() else {
        return [0.0; 3];
    };
    let mut total = [0.0_f64; 3];
    let mut count = 0u64;
    for face in faces {
        for pixel in face.chunks_exact(4) {
            total[0] += f64::from(pixel[0]);
            total[1] += f64::from(pixel[1]);
            total[2] += f64::from(pixel[2]);
            count += 1;
        }
    }
    if count == 0 {
        return [0.0; 3];
    }
    let count = count as f64;
    [
        (total[0] / count) as f32,
        (total[1] / count) as f32,
        (total[2] / count) as f32,
    ]
}

fn sanitize_environment_channel(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 64.0)
    } else {
        0.0
    }
}

fn sample_prefiltered_specular(
    cubemap: &PreparedEnvironmentCubemap,
    direction: Vec3,
    roughness: f32,
) -> Vec3 {
    let lod = prefilter_lod_for_roughness(roughness, cubemap.mip_count);
    sample_prefiltered_cubemap_lod(&cubemap.mips, direction, lod)
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbr_contribution_uses_prepared_diffuse_irradiance_not_raw_cubemap_radiance() {
        let black_face = vec![0.0, 0.0, 0.0, 1.0];
        let black_mip = [
            black_face.clone(),
            black_face.clone(),
            black_face.clone(),
            black_face.clone(),
            black_face.clone(),
            black_face,
        ];
        let lighting = PreparedEnvironmentLighting {
            diffuse_rgb: Vec3::new(0.5, 0.5, 0.5),
            specular_rgb: Vec3::ZERO,
            intensity: 1.0,
            rotation_y_radians: 0.0,
            cubemap: Some(Arc::new(PreparedEnvironmentCubemap {
                resolution: 1,
                mips: vec![black_mip],
                mip_count: 1,
                brdf_lut: vec![0.0, 0.0],
                brdf_lut_size: 1,
            })),
        };

        let contribution = lighting.pbr_contribution(
            PbrMaterial::new(Vec3::new(0.8, 0.7, 0.6), 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        assert!(
            contribution.x > 0.0 && contribution.y > 0.0 && contribution.z > 0.0,
            "diffuse IBL must use the prepared diffuse irradiance scalar; raw HDR cubemap \
             radiance can be black in the surface-normal direction and would leave this \
             dielectric material unlit"
        );
    }

    #[test]
    fn hdr_ibl_uses_calibrated_strength_for_diffuse_and_specular() {
        let desc = EnvironmentDesc::from_equirectangular_hdr_bytes(
            "memory://uniform-studio.hdr",
            &rle_radiance_hdr_uniform(8, 1, [64, 32, 16, 129]),
        )
        .expect("uniform HDR fixture decodes");
        let raw = desc
            .preview_irradiance_rgb()
            .expect("HDR decode records raw average radiance");
        assert_vec3_close(raw, [0.501_960_8, 0.250_980_4, 0.125_490_2]);

        let lighting = PreparedEnvironmentLighting::from_environment_with_profile(
            Some(&desc),
            EnvironmentLightingProfile::Reference,
        );

        assert_vec4_close(
            lighting.gpu_diffuse_intensity(),
            [0.501_960_8, 0.250_980_4, 0.125_490_2, 1.0],
        );
        assert_vec4_close(lighting.gpu_specular_intensity(), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn profile_mismatched_sidecar_does_not_silently_kill_ibl() {
        // A render that requests a profile the attached sidecar does not provide
        // (native `Reference` vs an `InteractiveWebGl2`-only prerendered sidecar)
        // must still be able to bake IBL specular from the retained equirect
        // source. Regression guard: a missing-profile sidecar previously zeroed
        // the prepared cubemap, flattening every chrome/metal reflection on
        // native renders (the "flat pale chrome" showcase bug).
        let path = "memory://uniform-studio.hdr";
        let bytes = rle_radiance_hdr_uniform(8, 1, [64, 32, 16, 129]);
        let plain = EnvironmentDesc::from_equirectangular_hdr_bytes(path, &bytes)
            .expect("uniform HDR fixture decodes")
            .with_cubemap_resolution(8);
        let sidecar =
            precompute_environment_sidecar(&plain, EnvironmentSidecarProfile::InteractiveWebGl2)
                .expect("interactive sidecar precomputes");
        let desc = EnvironmentDesc::from_equirectangular_hdr_sidecar_bytes(path, &bytes, sidecar)
            .expect("sidecar env constructs")
            .expect("sidecar sha matches source");

        assert!(
            desc.prefilter_sidecar(EnvironmentSidecarProfile::Reference)
                .is_none(),
            "precondition: a WebGl2 sidecar must not satisfy a Reference request"
        );
        assert!(
            desc.cubemap_faces().is_some(),
            "profile-mismatched sidecar must retain a bakeable equirect cubemap \
             source so native IBL specular does not silently go flat"
        );
    }

    #[test]
    fn profile_mismatched_sidecar_preserves_specular_reflection_contrast() {
        let path = "memory://striped-studio.hdr";
        let bytes = rle_radiance_hdr_vertical_stripes(16, 8, [8, 8, 8, 128], [255, 255, 255, 131]);
        let plain = EnvironmentDesc::from_equirectangular_hdr_bytes(path, &bytes)
            .expect("striped HDR fixture decodes")
            .with_cubemap_resolution(16);
        let sidecar =
            precompute_environment_sidecar(&plain, EnvironmentSidecarProfile::InteractiveWebGl2)
                .expect("interactive sidecar precomputes");
        let desc = EnvironmentDesc::from_equirectangular_hdr_sidecar_bytes(path, &bytes, sidecar)
            .expect("sidecar env constructs")
            .expect("sidecar sha matches source")
            .with_cubemap_resolution(16);

        let lighting = PreparedEnvironmentLighting::from_environment_with_profile(
            Some(&desc),
            EnvironmentLightingProfile::Reference,
        );
        let cubemap = lighting
            .cubemap()
            .expect("profile mismatch must bake a Reference cubemap from the HDR source");
        let directions = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.707, 0.0, 0.707),
            Vec3::new(-0.707, 0.0, 0.707),
            Vec3::new(0.707, 0.0, -0.707),
            Vec3::new(-0.707, 0.0, -0.707),
        ];
        let luminance_values = directions
            .into_iter()
            .map(|direction| luminance(sample_prefiltered_specular(cubemap, direction, 0.02)))
            .collect::<Vec<_>>();
        let min = luminance_values
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        let max = luminance_values
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);

        assert!(
            max > min + 0.2,
            "profile-mismatched sidecar fallback must preserve angular reflection contrast; \
             a constant specular fallback makes chrome flat. samples={luminance_values:?}"
        );
    }

    fn luminance(value: Vec3) -> f32 {
        value.x * 0.2126 + value.y * 0.7152 + value.z * 0.0722
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

    fn rle_radiance_hdr_vertical_stripes(
        width: u32,
        height: u32,
        dark: [u8; 4],
        bright: [u8; 4],
    ) -> Vec<u8> {
        assert!(width >= 8);
        assert!(width <= 127);
        let mut bytes =
            format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
        for _ in 0..height {
            bytes.push(0x02);
            bytes.push(0x02);
            bytes.push((width >> 8) as u8);
            bytes.push((width & 0xff) as u8);
            for channel in 0..4 {
                bytes.push(width as u8);
                for x in 0..width {
                    let source = if x < width / 4 || x >= width * 3 / 4 {
                        bright
                    } else {
                        dark
                    };
                    bytes.push(source[channel]);
                }
            }
        }
        bytes
    }

    fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
        for channel in 0..3 {
            assert!(
                (actual[channel] - expected[channel]).abs() < 0.001,
                "channel {channel}: expected {}, got {}",
                expected[channel],
                actual[channel]
            );
        }
    }

    fn assert_vec4_close(actual: [f32; 4], expected: [f32; 4]) {
        for channel in 0..4 {
            assert!(
                (actual[channel] - expected[channel]).abs() < 0.001,
                "channel {channel}: expected {}, got {}",
                expected[channel],
                actual[channel]
            );
        }
    }
}
