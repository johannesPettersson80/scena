use std::sync::Arc;

use crate::assets::EnvironmentDesc;
use crate::diagnostics::Backend;
use crate::scene::Vec3;

use super::environment_prefilter::{
    EnvironmentPrefilterQuality, build_brdf_lut_with_sample_count,
    prefilter_specular_cubemap_mips_with_quality,
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

/// Number of GGX-prefiltered specular mip levels emitted for the
/// environment cubemap. Mip 0 carries the source radiance; mips 1+
/// integrate the GGX kernel at increasing roughness so the WGSL
/// specular path can sample roughness via `prefilter_mip = roughness *
/// (mip_count - 1)`.
pub(in crate::render) const PREFILTER_MIP_COUNT: u32 = 5;
/// 2D BRDF LUT resolution. The split-sum approximation indexes the LUT
/// by `(N·V, roughness)`; 64×64 is enough resolution for visually
/// smooth specular without blowing the GPU upload budget.
pub(in crate::render) const BRDF_LUT_SIZE: u32 = 64;
const HDR_DIFFUSE_IBL_RESPONSE_SCALE: f32 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    fn prefilter_quality(self) -> EnvironmentPrefilterQuality {
        match self {
            Self::Reference => EnvironmentPrefilterQuality::Reference,
            Self::InteractiveWebGl2 => EnvironmentPrefilterQuality::InteractiveWebGl2,
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
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedEnvironmentLighting {
    diffuse_rgb: Vec3,
    specular_rgb: Vec3,
    intensity: f32,
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

        let cubemap_faces = environment.cubemap_faces();
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            environment_step_start = log_environment_step("cubemap_faces", environment_step_start);
        }
        let cubemap = cubemap_faces.map(|faces| {
            let resolution = faces.resolution();
            let source_pixels = faces.build_face_pixels_rgba32f();
            #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
            let prefilter_start =
                log_environment_step("build_face_pixels_rgba32f", environment_step_start);
            let mips = prefilter_specular_cubemap_mips_with_quality(
                &source_pixels,
                resolution,
                PREFILTER_MIP_COUNT,
                profile.prefilter_quality(),
            );
            #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
            let brdf_start =
                log_environment_step("prefilter_specular_cubemap_mips", prefilter_start);
            let brdf_lut = build_brdf_lut_with_sample_count(
                profile.brdf_lut_size(),
                profile.brdf_sample_count(),
            );
            #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
            {
                log_environment_step("build_brdf_lut", brdf_start);
            }
            Arc::new(PreparedEnvironmentCubemap {
                resolution,
                mips,
                mip_count: PREFILTER_MIP_COUNT,
                brdf_lut,
                brdf_lut_size: profile.brdf_lut_size(),
            })
        });
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
                cubemap,
            };
        }
        Self {
            diffuse_rgb,
            specular_rgb: Vec3::new(
                sanitize_environment_channel(irradiance[0]),
                sanitize_environment_channel(irradiance[1]),
                sanitize_environment_channel(irradiance[2]),
            ),
            intensity: 1.0,
            cubemap,
        }
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
        let reflection = reflect_vec3(Vec3::new(-view.x, -view.y, -view.z), normal);
        let prefiltered = self
            .cubemap
            .as_deref()
            .map(|cubemap| sample_prefiltered_specular(cubemap, reflection, material.roughness))
            .unwrap_or(self.specular_rgb);
        let brdf = self
            .cubemap
            .as_deref()
            .map(|cubemap| sample_brdf_lut(cubemap, dot_vec3(normal, view), material.roughness))
            .unwrap_or((1.0, 0.0));
        scale_vec3(
            environment_split_sum_contribution(material, normal, view, diffuse, prefiltered, brdf),
            self.intensity,
        )
    }
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
    let max_mip = cubemap.mip_count.saturating_sub(1);
    let mip = (roughness.clamp(0.0, 1.0) * max_mip as f32).round() as u32;
    sample_cubemap_mip(cubemap, mip, direction)
}

fn sample_cubemap_mip(cubemap: &PreparedEnvironmentCubemap, mip: u32, direction: Vec3) -> Vec3 {
    let Some(faces) = cubemap.mips.get(mip as usize) else {
        return Vec3::ZERO;
    };
    let resolution = (cubemap.resolution >> mip).max(1);
    let (face_index, u, v) = cubemap_face_uv(direction);
    let x = (u.clamp(0.0, 1.0) * (resolution - 1) as f32).round() as u32;
    let y = (v.clamp(0.0, 1.0) * (resolution - 1) as f32).round() as u32;
    let pixel = ((y * resolution + x) * 4) as usize;
    let face = &faces[face_index];
    if pixel + 2 >= face.len() {
        return Vec3::ZERO;
    }
    Vec3::new(face[pixel], face[pixel + 1], face[pixel + 2])
}

fn cubemap_face_uv(direction: Vec3) -> (usize, f32, f32) {
    let ax = direction.x.abs();
    let ay = direction.y.abs();
    let az = direction.z.abs();
    let (face, sc, tc, major) = if ax >= ay && ax >= az {
        if direction.x >= 0.0 {
            (0, -direction.z, -direction.y, ax)
        } else {
            (1, direction.z, -direction.y, ax)
        }
    } else if ay >= ax && ay >= az {
        if direction.y >= 0.0 {
            (2, direction.x, direction.z, ay)
        } else {
            (3, direction.x, -direction.z, ay)
        }
    } else if direction.z >= 0.0 {
        (4, direction.x, -direction.y, az)
    } else {
        (5, -direction.x, -direction.y, az)
    };
    if major <= f32::EPSILON || !major.is_finite() {
        return (4, 0.5, 0.5);
    }
    (face, 0.5 * (sc / major + 1.0), 0.5 * (tc / major + 1.0))
}

fn sample_brdf_lut(
    cubemap: &PreparedEnvironmentCubemap,
    n_dot_v: f32,
    roughness: f32,
) -> (f32, f32) {
    let size = cubemap.brdf_lut_size.max(1);
    let x = (n_dot_v.clamp(0.0, 1.0) * (size - 1) as f32).round() as u32;
    let y = (roughness.clamp(0.0, 1.0) * (size - 1) as f32).round() as u32;
    let index = ((y * size + x) * 2) as usize;
    if index + 1 >= cubemap.brdf_lut.len() {
        return (1.0, 0.0);
    }
    (cubemap.brdf_lut[index], cubemap.brdf_lut[index + 1])
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
    fn hdr_diffuse_ibl_uses_calibrated_strength_without_dimming_specular() {
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
            [0.401_568_65, 0.200_784_33, 0.100_392_16, 1.0],
        );
        assert_vec4_close(
            lighting.gpu_specular_intensity(),
            [0.501_960_8, 0.250_980_4, 0.125_490_2, 1.0],
        );
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
