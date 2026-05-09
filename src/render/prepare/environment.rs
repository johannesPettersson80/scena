use std::sync::Arc;

use crate::assets::EnvironmentDesc;
use crate::scene::Vec3;

use super::environment_prefilter::{build_brdf_lut, prefilter_specular_cubemap_mips};

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
    pub(in crate::render) fn from_environment(environment: Option<&EnvironmentDesc>) -> Self {
        let Some(environment) = environment else {
            return Self::default();
        };
        // Phase 1C step 1: parse the cubemap regardless of whether the CPU
        // shading path is going to consume scalar irradiance, so the GPU
        // pipeline can sample real per-fragment radiance. The scalar
        // diffuse/specular still come from `preview_irradiance_rgb` to keep
        // CPU rasterizer parity with the pre-Phase-1C fixtures.
        let cubemap_faces = environment.cubemap_faces();
        let cubemap = cubemap_faces.map(|faces| {
            let resolution = faces.resolution();
            let source_pixels = faces.build_face_pixels_rgba32f();
            let mips =
                prefilter_specular_cubemap_mips(&source_pixels, resolution, PREFILTER_MIP_COUNT);
            Arc::new(PreparedEnvironmentCubemap {
                resolution,
                mips,
                mip_count: PREFILTER_MIP_COUNT,
                brdf_lut: build_brdf_lut(BRDF_LUT_SIZE),
                brdf_lut_size: BRDF_LUT_SIZE,
            })
        });
        let Some(irradiance) = environment.preview_irradiance_rgb() else {
            return Self {
                diffuse_rgb: Vec3::ZERO,
                specular_rgb: Vec3::ZERO,
                intensity: 0.0,
                cubemap,
            };
        };
        let diffuse_rgb = Vec3::new(
            sanitize_environment_channel(irradiance[0]),
            sanitize_environment_channel(irradiance[1]),
            sanitize_environment_channel(irradiance[2]),
        );
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
            specular_rgb: scale_vec3(diffuse_rgb, 1.5),
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
        base: Vec3,
        metallic: f32,
        roughness: f32,
        normal: Vec3,
        view: Vec3,
    ) -> Vec3 {
        if !self.is_active() {
            return Vec3::ZERO;
        }
        let n_dot_v = dot_vec3(normal, view).max(0.001);
        let f0 = mix_vec3(Vec3::new(0.04, 0.04, 0.04), base, metallic);
        let fresnel = fresnel_schlick(n_dot_v, f0);
        let diffuse_energy = scale_vec3(
            subtract_vec3(Vec3::new(1.0, 1.0, 1.0), fresnel),
            1.0 - metallic,
        );
        let diffuse = multiply_vec3(multiply_vec3(diffuse_energy, base), self.diffuse_rgb);
        let specular_strength = (1.25 - roughness * 0.75).clamp(0.2, 1.25);
        let specular = scale_vec3(multiply_vec3(fresnel, self.specular_rgb), specular_strength);
        scale_vec3(add_vec3(diffuse, specular), self.intensity)
    }
}

pub(in crate::render) fn collect_environment_lighting(
    environment: Option<&EnvironmentDesc>,
) -> PreparedEnvironmentLighting {
    PreparedEnvironmentLighting::from_environment(environment)
}

fn sanitize_environment_channel(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 64.0)
    } else {
        0.0
    }
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn multiply_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x * right.x, left.y * right.y, left.z * right.z)
}

fn mix_vec3(left: Vec3, right: Vec3, amount: f32) -> Vec3 {
    let amount = if amount.is_finite() {
        amount.clamp(0.0, 1.0)
    } else {
        0.0
    };
    add_vec3(scale_vec3(left, 1.0 - amount), scale_vec3(right, amount))
}

fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    let factor = (1.0 - cos_theta.clamp(0.0, 1.0)).powi(5);
    add_vec3(
        f0,
        scale_vec3(subtract_vec3(Vec3::new(1.0, 1.0, 1.0), f0), factor),
    )
}
