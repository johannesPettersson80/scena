use super::super::pbr_contract::{
    PbrMaterial, anisotropy_light_contribution, clearcoat_light_contribution,
    dispersion_light_contribution, iridescence_light_contribution, sheen_light_contribution,
    transmission_volume_light_contribution,
};
use super::math::add_vec3;
use crate::scene::Vec3;

#[derive(Clone, Copy)]
pub(super) struct LayeredMaterialLobes {
    pub(super) pbr_material: PbrMaterial,
    pub(super) normal: Vec3,
    pub(super) clearcoat_normal: Vec3,
    pub(super) view: Vec3,
    pub(super) tangent: Vec3,
    pub(super) tangent_handedness: f32,
    pub(super) clearcoat_factor: f32,
    pub(super) clearcoat_roughness: f32,
    pub(super) sheen_color: Vec3,
    pub(super) sheen_roughness: f32,
    pub(super) anisotropy_strength: f32,
    pub(super) anisotropy_rotation: f32,
    pub(super) anisotropy_texture: Vec3,
    pub(super) iridescence_factor: f32,
    pub(super) iridescence_ior: f32,
    pub(super) iridescence_thickness_minimum: f32,
    pub(super) iridescence_thickness_maximum: f32,
    pub(super) iridescence_texture: f32,
    pub(super) iridescence_thickness_texture: f32,
    pub(super) dispersion_factor: f32,
    pub(super) transmission_factor: f32,
    pub(super) ior: f32,
    pub(super) thickness_factor: f32,
    pub(super) thickness_texture: f32,
    pub(super) attenuation_color: Vec3,
    pub(super) attenuation_distance: f32,
}

impl LayeredMaterialLobes {
    pub(super) fn contribution(self, incoming: Vec3, radiance: Vec3) -> Vec3 {
        let mut contribution = Vec3::ZERO;
        contribution = add_vec3(
            contribution,
            clearcoat_light_contribution(
                self.clearcoat_normal,
                self.view,
                incoming,
                radiance,
                self.clearcoat_factor,
                self.clearcoat_roughness,
            ),
        );
        contribution = add_vec3(
            contribution,
            sheen_light_contribution(
                self.normal,
                self.view,
                incoming,
                radiance,
                self.sheen_color,
                self.sheen_roughness,
            ),
        );
        contribution = add_vec3(
            contribution,
            anisotropy_light_contribution(
                self.pbr_material,
                self.normal,
                self.tangent,
                self.tangent_handedness,
                self.view,
                incoming,
                radiance,
                self.anisotropy_strength,
                self.anisotropy_rotation,
                self.anisotropy_texture,
            ),
        );
        contribution = add_vec3(
            contribution,
            iridescence_light_contribution(
                self.pbr_material,
                self.normal,
                self.view,
                incoming,
                radiance,
                self.iridescence_factor,
                self.iridescence_ior,
                self.iridescence_thickness_minimum,
                self.iridescence_thickness_maximum,
                self.iridescence_texture,
                self.iridescence_thickness_texture,
            ),
        );
        contribution = add_vec3(
            contribution,
            dispersion_light_contribution(
                self.pbr_material,
                self.normal,
                self.view,
                incoming,
                radiance,
                self.dispersion_factor,
                self.ior,
            ),
        );
        add_vec3(
            contribution,
            transmission_volume_light_contribution(
                self.pbr_material,
                self.normal,
                self.view,
                incoming,
                radiance,
                self.transmission_factor,
                self.ior,
                self.thickness_factor,
                self.thickness_texture,
                self.attenuation_color,
                self.attenuation_distance,
            ),
        )
    }
}
