//! Image-based lighting of the layered surface model: the Rust side of
//! `surface_environment_light` in `src/render/gpu/surface_lighting.wgsl`.

use crate::scene::Vec3;

use super::super::super::pbr_brdf;
use super::super::pbr_contract::{Surface, reflect_vec3, sheen_environment_brdf};
use super::{
    PreparedEnvironmentLighting, dot_vec3, rotate_environment_y, sample_prefiltered_specular,
    scale_vec3,
};

impl PreparedEnvironmentLighting {
    /// Image-based lighting for every layer of `s`.
    ///
    /// The layers follow the shader: anisotropy bends the reflection,
    /// iridescence drives the integrated reflectance, sheen and clearcoat sit
    /// on top. The base layer keeps the form the CPU reference images were
    /// approved with: the analytic split-sum fit, Schlick diffuse weighting at
    /// the view angle, and single scattering only. The shader reads the baked
    /// table for the base as well and adds the Fdez-Aguera multiple-scattering
    /// term; moving the base here changes every image-based-lit material,
    /// layered or not.
    pub(in crate::render::prepare) fn surface_environment_light(&self, s: &Surface) -> Vec3 {
        if !self.is_active() {
            return Vec3::ZERO;
        }
        let irradiance = self.diffuse_radiance(s.normal);
        let prefiltered = self.environment_prefiltered(s.view, s.reflection_normal(), s.roughness);
        let lut = if self.cubemap.is_some() {
            pbr_brdf::split_sum_brdf_approx(dot_vec3(s.normal, s.view), s.roughness)
        } else {
            (1.0, 0.0)
        };
        let f0 = s.environment_f0();
        let fresnel = pbr_brdf::fresnel_schlick(f0, s.n_dot_v);
        let diffuse_energy = scale_vec3(Vec3::new(1.0, 1.0, 1.0) - fresnel, 1.0 - s.metallic);
        let diffuse = diffuse_energy * s.base * irradiance;
        let single_scatter = scale_vec3(f0, lut.0) + Vec3::new(lut.1, lut.1, lut.1);
        let mut lit = diffuse + prefiltered * single_scatter;
        if s.has_sheen() {
            lit = scale_vec3(lit, s.sheen_scaling)
                + scale_vec3(
                    s.sheen_color * irradiance,
                    sheen_environment_brdf(s.n_dot_v, s.sheen_roughness),
                );
        }
        if s.clearcoat > 0.0 {
            let coat_n_dot_v = dot_vec3(s.clearcoat_normal, s.view).max(pbr_brdf::MIN_N_DOT_V);
            let coat_prefiltered =
                self.environment_prefiltered(s.view, s.clearcoat_normal, s.clearcoat_roughness);
            let coat_lut = self.split_sum_brdf_table(coat_n_dot_v, s.clearcoat_roughness);
            lit = scale_vec3(lit, s.clearcoat_attenuation)
                + scale_vec3(
                    coat_prefiltered,
                    (pbr_brdf::DIELECTRIC_F0 * coat_lut.0 + coat_lut.1) * s.clearcoat,
                );
        }
        scale_vec3(lit, self.intensity)
    }

    /// Prefiltered radiance arriving along the mirror direction of `view`
    /// about `normal`, or the scalar specular colour when no cubemap exists.
    fn environment_prefiltered(&self, view: Vec3, normal: Vec3, roughness: f32) -> Vec3 {
        let reflection = rotate_environment_y(
            reflect_vec3(Vec3::new(-view.x, -view.y, -view.z), normal),
            self.rotation_y_radians,
        );
        self.cubemap
            .as_deref()
            .map(|cubemap| sample_prefiltered_specular(cubemap, reflection, roughness))
            .unwrap_or(self.specular_rgb)
    }

    /// The baked split-sum table the shader reads, or the scalar fallback
    /// when no cubemap exists.
    fn split_sum_brdf_table(&self, n_dot_v: f32, roughness: f32) -> (f32, f32) {
        self.cubemap.as_deref().map_or((1.0, 0.0), |cubemap| {
            pbr_brdf::split_sum_brdf_table(
                &cubemap.brdf_lut,
                cubemap.brdf_lut_size,
                n_dot_v,
                roughness,
            )
        })
    }
}

#[cfg(test)]
mod tests;
