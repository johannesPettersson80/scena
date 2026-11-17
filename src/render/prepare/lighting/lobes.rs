use super::super::pbr_contract::{PbrMaterial, dispersion_light_contribution};
use super::math::add_vec3;
use crate::scene::Vec3;

/// The dispersion response. The layered surface (`pbr_contract::Surface`)
/// does not model it; it is added on top with the CPU renderer's own
/// approximation.
#[derive(Clone, Copy)]
pub(super) struct VolumeLobes {
    /// The material with its full base colour, which dispersion works from.
    pub(super) pbr_material: PbrMaterial,
    /// The material with the base colour a transmissive surface keeps for
    /// itself, which the surface and the area-light specular lobe use.
    pub(super) opaque_material: PbrMaterial,
    pub(super) normal: Vec3,
    pub(super) view: Vec3,
    pub(super) dispersion_factor: f32,
    pub(super) ior: f32,
}

impl VolumeLobes {
    pub(super) fn contribution(self, incoming: Vec3, radiance: Vec3) -> Vec3 {
        // Summed from zero, as it always was, so the output keeps its bits.
        // Light that passes through a transmissive surface is the scene
        // colour behind it, added by the transmission pass; no lobe here
        // stands in for it.
        add_vec3(
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
        )
    }
}
