use super::super::pbr_contract::{
    PbrMaterial, dispersion_light_contribution, transmission_volume_light_contribution,
};
use super::math::add_vec3;
use crate::scene::Vec3;

/// Dispersion and transmission-volume responses. The layered surface
/// (`pbr_contract::Surface`) does not model them; they are added on top of it
/// with the CPU renderer's own approximations.
#[derive(Clone, Copy)]
pub(super) struct VolumeLobes {
    pub(super) pbr_material: PbrMaterial,
    pub(super) normal: Vec3,
    pub(super) view: Vec3,
    pub(super) dispersion_factor: f32,
    pub(super) transmission_factor: f32,
    pub(super) ior: f32,
    pub(super) thickness_factor: f32,
    pub(super) thickness_texture: f32,
    pub(super) attenuation_color: Vec3,
    pub(super) attenuation_distance: f32,
}

impl VolumeLobes {
    pub(super) fn contribution(self, incoming: Vec3, radiance: Vec3) -> Vec3 {
        // Summed from zero, as these two lobes always were, so their output
        // keeps its bits.
        let contribution = add_vec3(
            Vec3::ZERO,
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
