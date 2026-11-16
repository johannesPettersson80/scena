//! Layered glTF surface model: the Rust side of
//! `src/render/gpu/surface_lighting.wgsl`.
//!
//! One [`Surface`] is built per shaded vertex and every light and the
//! environment are evaluated against it, so the layering rules live in one
//! place. Fields and functions keep their WGSL names so the two files can be
//! read side by side.
//!
//! Layering follows the Khronos glTF material extensions:
//!   - base: Lambert diffuse + GGX specular (KHR_materials_ior), with
//!     anisotropy replacing the specular distribution and iridescence
//!     replacing its Fresnel term,
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

use super::anisotropy::{
    anisotropic_reflection_normal, distribution_ggx_anisotropic, surface_anisotropy_tangent,
    visibility_ggx_anisotropic,
};
use super::iridescence::iridescence_fresnel;
use super::sheen::{
    MIN_SHEEN_ROUGHNESS, SHEEN_ENERGY_COMPENSATION, distribution_charlie, visibility_neubelt,
};
use super::{
    DEFAULT_IOR, DIELECTRIC_F0, MIN_DENOMINATOR, MIN_N_DOT_V, PbrMaterial, add_vec3, clamp_unit,
    clamp_vec3_unit, cross_vec3, dot_vec3, f0_from_ior, fresnel_schlick, max_component, mix_scalar,
    multiply_vec3, normalize_or, pbr_brdf, roughness_or_min, scale_vec3, subtract_vec3,
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
    pub(in crate::render::prepare) anisotropy: f32,
    pub(in crate::render::prepare) anisotropy_t: Vec3,
    pub(in crate::render::prepare) anisotropy_b: Vec3,
    pub(in crate::render::prepare) iridescence: f32,
    pub(in crate::render::prepare) iridescence_fresnel: Vec3,
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
            anisotropy: 0.0,
            anisotropy_t: Vec3::ZERO,
            anisotropy_b: Vec3::ZERO,
            iridescence: 0.0,
            iridescence_fresnel: Vec3::ZERO,
        }
    }

    /// Sets the dielectric reflectance from KHR_materials_ior.
    ///
    /// The glTF default index of 1.5 is the 0.04 the base layer is built with
    /// and leaves the surface untouched. Call this before
    /// [`Self::with_iridescence`], which is computed from the reflectance.
    pub(in crate::render::prepare) fn with_ior(mut self, ior: f32) -> Self {
        debug_assert!(
            self.iridescence <= 0.0,
            "the index of refraction must be set before the iridescence layer"
        );
        if ior.is_finite() && ior != DEFAULT_IOR {
            self.f0 = pbr_brdf::f0_for_dielectric(f0_from_ior(ior), self.base, self.metallic);
        }
        self
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

    /// Reshapes the base specular lobe per KHR_materials_anisotropy. A zero
    /// strength leaves the surface untouched.
    ///
    /// `direction` is the authored tangent-space direction, `rotation` the
    /// authored rotation in radians, and `tangent` the mesh tangent.
    pub(in crate::render::prepare) fn with_anisotropy(
        mut self,
        strength: f32,
        direction: (f32, f32),
        rotation: f32,
        tangent: Vec3,
        tangent_handedness: f32,
    ) -> Self {
        let strength = clamp_unit(strength);
        if strength > 0.0 {
            self.anisotropy = strength;
            self.anisotropy_t = surface_anisotropy_tangent(
                self.normal,
                tangent,
                tangent_handedness,
                direction,
                rotation,
            );
            self.anisotropy_b = normalize_or(
                cross_vec3(self.normal, self.anisotropy_t),
                self.anisotropy_t,
            );
        }
        self
    }

    /// Replaces the base Fresnel term with thin-film interference per
    /// KHR_materials_iridescence. A zero factor leaves the surface untouched.
    pub(in crate::render::prepare) fn with_iridescence(
        mut self,
        factor: f32,
        film_ior: f32,
        thickness_nm: f32,
    ) -> Self {
        let factor = clamp_unit(factor);
        if factor > 0.0 {
            self.iridescence = factor;
            self.iridescence_fresnel =
                iridescence_fresnel(film_ior, self.n_dot_v, thickness_nm, self.f0);
        }
        self
    }

    pub(in crate::render::prepare) fn has_sheen(&self) -> bool {
        max_component(self.sheen_color) > 0.0
    }

    /// Normal the environment is mirrored about. An anisotropic surface bends
    /// it toward the anisotropic bitangent so the reflection stretches along
    /// the grooves.
    pub(in crate::render::prepare) fn reflection_normal(&self) -> Vec3 {
        if self.anisotropy > 0.0 {
            return anisotropic_reflection_normal(
                self.normal,
                self.view,
                self.anisotropy_b,
                self.anisotropy,
                self.roughness,
            );
        }
        self.normal
    }

    /// Reflectance at normal incidence that image-based lighting integrates.
    /// With iridescence, Schlick is inverted at the view angle so the film
    /// colour drives the integrated reflectance too.
    pub(in crate::render::prepare) fn environment_f0(&self) -> Vec3 {
        if self.iridescence <= 0.0 {
            return self.f0;
        }
        let fresnel_weight = pow5(1.0 - self.n_dot_v);
        let scale = (1.0 - fresnel_weight).max(MIN_DENOMINATOR).recip();
        let film_f0 = clamp_vec3_unit(scale_vec3(
            subtract_vec3(
                self.iridescence_fresnel,
                Vec3::new(fresnel_weight, fresnel_weight, fresnel_weight),
            ),
            scale,
        ));
        mix_vec3(self.f0, film_f0, self.iridescence)
    }
}

/// Base-layer Fresnel, with thin-film iridescence replacing the Schlick term.
fn surface_fresnel(s: &Surface, v_dot_h: f32) -> Vec3 {
    let schlick = fresnel_schlick(v_dot_h, s.f0);
    if s.iridescence > 0.0 {
        return mix_vec3(schlick, s.iridescence_fresnel, s.iridescence);
    }
    schlick
}

fn surface_anisotropic_specular(
    s: &Surface,
    incoming: Vec3,
    half_vector: Vec3,
    n_dot_l: f32,
    n_dot_h: f32,
) -> f32 {
    let tangent_alpha = mix_scalar(s.alpha, 1.0, s.anisotropy * s.anisotropy);
    let distribution = distribution_ggx_anisotropic(
        n_dot_h,
        dot_vec3(s.anisotropy_t, half_vector),
        dot_vec3(s.anisotropy_b, half_vector),
        tangent_alpha,
        s.alpha,
    );
    let visibility = visibility_ggx_anisotropic(
        n_dot_l,
        s.n_dot_v,
        dot_vec3(s.anisotropy_b, s.view),
        dot_vec3(s.anisotropy_t, s.view),
        dot_vec3(s.anisotropy_t, incoming),
        dot_vec3(s.anisotropy_b, incoming),
        tangent_alpha,
        s.alpha,
    );
    distribution * visibility
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
        let fresnel = surface_fresnel(s, v_dot_h);
        let diffuse_energy = scale_vec3(
            subtract_vec3(Vec3::new(1.0, 1.0, 1.0), fresnel),
            1.0 - s.metallic,
        );
        let diffuse = scale_vec3(multiply_vec3(diffuse_energy, s.base), PI.recip());
        let mut layer = diffuse;
        if base_specular_weight > 0.0 {
            let mut specular_brdf =
                pbr_brdf::brdf_specular_ggx(s.alpha, n_dot_l, s.n_dot_v, n_dot_h);
            if s.anisotropy > 0.0 {
                specular_brdf =
                    surface_anisotropic_specular(s, incoming, half_vector, n_dot_l, n_dot_h);
            }
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

fn mix_vec3(left: Vec3, right: Vec3, amount: f32) -> Vec3 {
    add_vec3(scale_vec3(left, 1.0 - amount), scale_vec3(right, amount))
}

fn pow5(value: f32) -> f32 {
    let squared = value * value;
    squared * squared * value
}

#[cfg(test)]
mod tests;
