//! Layered glTF surface model: the Rust side of
//! `src/render/gpu/surface_lighting.wgsl`.
//!
//! One [`Surface`] is built per shaded vertex and every light and the
//! environment are evaluated against it, so the layering rules live in one
//! place. Fields and functions keep their WGSL names so the two files can be
//! read side by side.
//!
//! Layering follows the Khronos glTF material extensions:
//!   - base: Lambert diffuse + GGX specular,
//!   - sheen: Charlie lobe on top, base scaled down by the energy it reflects,
//!   - clearcoat: GGX lobe on top, everything below attenuated by its Fresnel.
//!
//! A layer's code runs only when that layer is present. Without one the
//! arithmetic is the core metallic-roughness BRDF operation for operation,
//! which keeps materials without extensions on the output their reference
//! images were approved with. That is also why the base terms are written in
//! the CPU renderer's own evaluation order rather than the shader's: the two
//! are the same formula, but only one of them reproduces those images bit for
//! bit. The WGSL carries `f90` and `diffuse_color` as fields; here `f90` is the
//! constant 1 inside `fresnel_schlick` and `diffuse_color` is expanded in place.

use std::f32::consts::PI;

use crate::scene::Vec3;

use super::sheen::{
    MIN_SHEEN_ROUGHNESS, SHEEN_ENERGY_COMPENSATION, distribution_charlie, visibility_neubelt,
};
use super::{
    DIELECTRIC_F0, MIN_N_DOT_V, PbrMaterial, add_vec3, clamp_unit, clamp_vec3_unit, dot_vec3,
    fresnel_schlick, max_component, multiply_vec3, normalize_or, pbr_brdf, roughness_or_min,
    scale_vec3, subtract_vec3,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render::prepare) struct Surface {
    pub(in crate::render::prepare) base: Vec3,
    pub(in crate::render::prepare) metallic: f32,
    pub(in crate::render::prepare) f0: Vec3,
    pub(in crate::render::prepare) roughness: f32,
    pub(in crate::render::prepare) alpha: f32,
    pub(in crate::render::prepare) normal: Vec3,
    pub(in crate::render::prepare) view: Vec3,
    pub(in crate::render::prepare) n_dot_v: f32,
    pub(in crate::render::prepare) clearcoat: f32,
    pub(in crate::render::prepare) clearcoat_roughness: f32,
    pub(in crate::render::prepare) clearcoat_normal: Vec3,
    /// Fraction of light from the layers below that survives the clearcoat.
    pub(in crate::render::prepare) clearcoat_attenuation: f32,
    pub(in crate::render::prepare) sheen_color: Vec3,
    pub(in crate::render::prepare) sheen_roughness: f32,
    /// Fraction of base-layer energy left after the sheen layer reflects.
    pub(in crate::render::prepare) sheen_scaling: f32,
}

impl Surface {
    /// The base layer alone: a core metallic-roughness surface.
    ///
    /// `normal` and `view` are unit vectors in the same space.
    pub(in crate::render::prepare) fn new(material: PbrMaterial, normal: Vec3, view: Vec3) -> Self {
        Self {
            base: material.base,
            metallic: material.metallic,
            f0: material.f0(),
            roughness: material.roughness,
            alpha: pbr_brdf::alpha_roughness(material.roughness),
            normal,
            view,
            n_dot_v: dot_vec3(normal, view).max(MIN_N_DOT_V),
            clearcoat: 0.0,
            clearcoat_roughness: roughness_or_min(0.0),
            clearcoat_normal: normal,
            clearcoat_attenuation: 1.0,
            sheen_color: Vec3::ZERO,
            sheen_roughness: MIN_SHEEN_ROUGHNESS,
            sheen_scaling: 1.0,
        }
    }

    /// Adds a KHR_materials_clearcoat layer. A zero factor leaves the surface
    /// untouched.
    pub(in crate::render::prepare) fn with_clearcoat(
        mut self,
        factor: f32,
        roughness: f32,
        normal: Vec3,
    ) -> Self {
        let factor = clamp_unit(factor);
        if factor > 0.0 {
            let coat_fresnel = fresnel_schlick(
                dot_vec3(normal, self.view).max(0.0),
                Vec3::new(DIELECTRIC_F0, DIELECTRIC_F0, DIELECTRIC_F0),
            );
            self.clearcoat = factor;
            self.clearcoat_roughness = roughness_or_min(roughness);
            self.clearcoat_normal = normal;
            self.clearcoat_attenuation = 1.0 - factor * coat_fresnel.x;
        }
        self
    }

    /// Adds a KHR_materials_sheen layer. A black sheen colour leaves the
    /// surface untouched.
    pub(in crate::render::prepare) fn with_sheen(mut self, color: Vec3, roughness: f32) -> Self {
        let color = clamp_vec3_unit(color);
        if max_component(color) > 0.0 {
            self.sheen_color = color;
            self.sheen_roughness = roughness_or_min(roughness).max(MIN_SHEEN_ROUGHNESS);
            self.sheen_scaling = 1.0 - SHEEN_ENERGY_COMPENSATION * max_component(color);
        }
        self
    }

    pub(in crate::render::prepare) fn has_sheen(&self) -> bool {
        max_component(self.sheen_color) > 0.0
    }
}

/// Radiance reflected toward the viewer from one light.
/// `base_specular_weight` is 0 for area-light point samples, whose base
/// specular lobe is integrated analytically by the LTC path instead.
pub(in crate::render::prepare) fn surface_direct_light(
    s: &Surface,
    incoming: Vec3,
    radiance: Vec3,
    base_specular_weight: f32,
) -> Vec3 {
    let incoming = normalize_or(incoming, Vec3::ZERO);
    let half_vector = normalize_or(add_vec3(s.view, incoming), s.normal);
    let v_dot_h = dot_vec3(s.view, half_vector).max(0.0);
    let mut lit = Vec3::ZERO;
    let n_dot_l = dot_vec3(s.normal, incoming).max(0.0);
    if n_dot_l > f32::EPSILON {
        let n_dot_h = dot_vec3(s.normal, half_vector).max(0.0);
        let fresnel = fresnel_schlick(v_dot_h, s.f0);
        let diffuse_energy = scale_vec3(
            subtract_vec3(Vec3::new(1.0, 1.0, 1.0), fresnel),
            1.0 - s.metallic,
        );
        let diffuse = scale_vec3(multiply_vec3(diffuse_energy, s.base), PI.recip());
        let mut layer = diffuse;
        if base_specular_weight > 0.0 {
            let specular_brdf = pbr_brdf::brdf_specular_ggx(s.alpha, n_dot_l, s.n_dot_v, n_dot_h);
            layer = add_vec3(
                diffuse,
                scale_vec3(fresnel, specular_brdf * base_specular_weight),
            );
        }
        if s.has_sheen() {
            let sheen_alpha = s.sheen_roughness * s.sheen_roughness;
            layer = add_vec3(
                scale_vec3(layer, s.sheen_scaling),
                scale_vec3(
                    s.sheen_color,
                    distribution_charlie(sheen_alpha, n_dot_h)
                        * visibility_neubelt(n_dot_l, s.n_dot_v),
                ),
            );
        }
        lit = scale_vec3(
            multiply_vec3(layer, radiance),
            n_dot_l * s.clearcoat_attenuation,
        );
    }
    if s.clearcoat > 0.0 {
        let coat_n_dot_l = dot_vec3(s.clearcoat_normal, incoming).max(0.0);
        if coat_n_dot_l > 0.0 {
            let coat_n_dot_v = dot_vec3(s.clearcoat_normal, s.view).max(MIN_N_DOT_V);
            let coat_n_dot_h = dot_vec3(s.clearcoat_normal, half_vector).max(0.0);
            let coat_alpha = s.clearcoat_roughness * s.clearcoat_roughness;
            let coat_brdf =
                pbr_brdf::brdf_specular_ggx(coat_alpha, coat_n_dot_l, coat_n_dot_v, coat_n_dot_h);
            let coat_fresnel = fresnel_schlick(
                v_dot_h,
                Vec3::new(DIELECTRIC_F0, DIELECTRIC_F0, DIELECTRIC_F0),
            );
            lit = add_vec3(
                lit,
                scale_vec3(
                    multiply_vec3(scale_vec3(coat_fresnel, coat_brdf), radiance),
                    coat_n_dot_l * s.clearcoat,
                ),
            );
        }
    }
    lit
}

#[cfg(test)]
mod tests;
