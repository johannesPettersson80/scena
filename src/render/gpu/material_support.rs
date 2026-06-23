use crate::PrepareError;
use crate::diagnostics::Backend;
use crate::render::prepare::PreparedMaterialSlot;

use super::super::RasterTarget;

pub(super) fn reject_unsupported_volume_texture_slots(
    target: RasterTarget,
    slots: &[PreparedMaterialSlot],
) -> Result<(), PrepareError> {
    if !matches!(
        target.backend,
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2
    ) {
        return Ok(());
    }
    if slots
        .iter()
        .any(|slot| slot.transmission.is_some() || slot.thickness.is_some())
    {
        return Err(PrepareError::BackendCapabilityMismatch {
            feature: "gpu_volume_texture_slots",
            backend: target.backend,
            help: "transmission_texture and thickness_texture are not bound on the GPU/WebGL2 path yet; use scalar transmission_factor, ior, thickness_factor, attenuation_distance, and attenuation_color for recipe-supported glass".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetPath, MaterialHandle, TextureDesc, TextureSourceFormat};
    use crate::material::{Color, MaterialDesc, TextureColorSpace};
    use crate::render::prepare::PreparedMaterialTexture;

    #[test]
    fn gpu_rejects_volume_texture_slots_before_silent_drop() {
        let slot = material_slot_with_transmission_texture();
        let error = reject_unsupported_volume_texture_slots(
            target(Backend::HeadlessGpu),
            std::slice::from_ref(&slot),
        )
        .expect_err("GPU path must fail closed for unbound volume texture slots");

        assert!(matches!(
            error,
            PrepareError::BackendCapabilityMismatch {
                feature: "gpu_volume_texture_slots",
                backend: Backend::HeadlessGpu,
                ..
            }
        ));
        assert!(
            reject_unsupported_volume_texture_slots(target(Backend::Headless), &[slot]).is_ok()
        );
    }

    #[test]
    fn gpu_accepts_scalar_transmission_without_volume_texture_slots() {
        let slot = PreparedMaterialSlot {
            handle: MaterialHandle::default(),
            material: MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.1)
                .with_transmission_factor(1.0)
                .with_thickness_factor(0.4),
            base_color: None,
            normal: None,
            metallic_roughness: None,
            occlusion: None,
            emissive: None,
            clearcoat: None,
            clearcoat_roughness: None,
            clearcoat_normal: None,
            sheen_color: None,
            sheen_roughness: None,
            anisotropy: None,
            iridescence: None,
            iridescence_thickness: None,
            transmission: None,
            thickness: None,
        };

        assert!(
            reject_unsupported_volume_texture_slots(target(Backend::HeadlessGpu), &[slot]).is_ok()
        );
    }

    fn target(backend: Backend) -> RasterTarget {
        RasterTarget {
            width: 32,
            height: 32,
            backend,
        }
    }

    fn material_slot_with_transmission_texture() -> PreparedMaterialSlot {
        PreparedMaterialSlot {
            handle: MaterialHandle::default(),
            material: MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.1)
                .with_transmission_factor(1.0),
            base_color: None,
            normal: None,
            metallic_roughness: None,
            occlusion: None,
            emissive: None,
            clearcoat: None,
            clearcoat_roughness: None,
            clearcoat_normal: None,
            sheen_color: None,
            sheen_roughness: None,
            anisotropy: None,
            iridescence: None,
            iridescence_thickness: None,
            transmission: Some(PreparedMaterialTexture {
                handle: Default::default(),
                desc: texture_desc(),
                transform: None,
            }),
            thickness: None,
        }
    }

    fn texture_desc() -> TextureDesc {
        TextureDesc::new_with_bytes(
            AssetPath::from("memory://gpu-volume-texture.png"),
            TextureColorSpace::Linear,
            Default::default(),
            TextureSourceFormat::Png,
            None,
        )
        .expect("test texture descriptor builds")
    }
}
