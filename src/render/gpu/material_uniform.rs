use crate::material::{AlphaMode, MaterialDesc, MaterialKind, TextureTransform};

pub(super) const MATERIAL_UNIFORM_BYTE_LEN: u64 = 80;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MaterialUniformUpload {
    pub(super) offset_scale: [f32; 4],
    pub(super) rotation: [f32; 4],
    pub(super) base_color_factor: [f32; 4],
    pub(super) emissive_strength: [f32; 4],
    pub(super) metallic_roughness_alpha: [f32; 4],
}

impl MaterialUniformUpload {
    pub(super) fn from_material(
        material: Option<&MaterialDesc>,
        base_color_transform: Option<TextureTransform>,
    ) -> Self {
        let transform = Self::from_transform(base_color_transform);
        let Some(material) = material else {
            return transform;
        };
        let alpha_cutoff = match material.alpha_mode() {
            AlphaMode::Mask { cutoff } => cutoff,
            AlphaMode::Opaque | AlphaMode::Blend => 0.0,
        };
        let unlit_flag = match material.kind() {
            MaterialKind::Unlit => 1.0,
            MaterialKind::PbrMetallicRoughness
            | MaterialKind::Line
            | MaterialKind::Wireframe
            | MaterialKind::Edge => 0.0,
        };
        Self {
            offset_scale: transform.offset_scale,
            rotation: transform.rotation,
            base_color_factor: [
                material.base_color().r,
                material.base_color().g,
                material.base_color().b,
                material.base_color().a,
            ],
            emissive_strength: [
                material.emissive().r,
                material.emissive().g,
                material.emissive().b,
                material.emissive_strength(),
            ],
            metallic_roughness_alpha: [
                material.metallic_factor(),
                material.roughness_factor(),
                alpha_cutoff,
                unlit_flag,
            ],
        }
    }

    pub(super) fn from_transform(transform: Option<TextureTransform>) -> Self {
        let Some(transform) = transform else {
            return Self::identity();
        };
        let rotation = transform.rotation_radians();
        Self {
            offset_scale: [
                transform.offset()[0],
                transform.offset()[1],
                transform.scale()[0],
                transform.scale()[1],
            ],
            rotation: [rotation.sin(), rotation.cos(), 0.0, 0.0],
            ..Self::identity()
        }
    }

    pub(super) fn identity() -> Self {
        Self {
            offset_scale: [0.0, 0.0, 1.0, 1.0],
            rotation: [0.0, 1.0, 0.0, 0.0],
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            emissive_strength: [0.0, 0.0, 0.0, 1.0],
            metallic_roughness_alpha: [0.0, 1.0, 0.0, 0.0],
        }
    }

    pub(super) fn encode(self) -> [u8; MATERIAL_UNIFORM_BYTE_LEN as usize] {
        let mut bytes = [0; MATERIAL_UNIFORM_BYTE_LEN as usize];
        for (index, value) in self
            .offset_scale
            .into_iter()
            .chain(self.rotation)
            .chain(self.base_color_factor)
            .chain(self.emissive_strength)
            .chain(self.metallic_roughness_alpha)
            .enumerate()
        {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{MATERIAL_UNIFORM_BYTE_LEN, MaterialUniformUpload};
    use crate::material::{AlphaMode, Color, MaterialDesc, TextureTransform};

    #[test]
    fn material_uniform_upload_encodes_base_color_texture_transform() {
        let transform = TextureTransform::new([0.25, 0.5], 0.5, [0.75, 0.5], None);
        let upload = MaterialUniformUpload::from_transform(Some(transform));

        assert_eq!(upload.offset_scale, [0.25, 0.5, 0.75, 0.5]);
        assert!((upload.rotation[0] - 0.5_f32.sin()).abs() < f32::EPSILON);
        assert!((upload.rotation[1] - 0.5_f32.cos()).abs() < f32::EPSILON);
        assert_eq!(upload.encode().len(), MATERIAL_UNIFORM_BYTE_LEN as usize);
    }

    #[test]
    fn material_uniform_upload_encodes_material_factors() {
        let material = MaterialDesc::pbr_metallic_roughness(
            Color::from_linear_rgba(0.2, 0.4, 0.6, 0.8),
            0.3,
            0.7,
        )
        .with_emissive(Color::from_linear_rgba(0.1, 0.2, 0.3, 1.0))
        .with_emissive_strength(2.5)
        .with_alpha_mode(AlphaMode::Mask { cutoff: 0.45 });

        let upload = MaterialUniformUpload::from_material(Some(&material), None);

        assert_eq!(upload.base_color_factor, [0.2, 0.4, 0.6, 0.8]);
        assert_eq!(upload.emissive_strength, [0.1, 0.2, 0.3, 2.5]);
        assert_eq!(upload.metallic_roughness_alpha, [0.3, 0.7, 0.45, 0.0]);
        assert_eq!(
            upload.encode().len(),
            80,
            "material uniform must reserve transform, base color, emissive, metallic, \
             roughness, and alpha-mask factor lanes"
        );
    }

    #[test]
    fn material_uniform_upload_marks_unlit_materials() {
        let material = MaterialDesc::unlit(Color::WHITE);
        let upload = MaterialUniformUpload::from_material(Some(&material), None);

        assert_eq!(upload.metallic_roughness_alpha[3], 1.0);
    }
}
