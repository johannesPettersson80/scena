use crate::material::{AlphaMode, MaterialDesc, MaterialKind, TextureTransform};

/// Plan line 778 / RFC 866 commit 2: the MaterialUniform now carries a
/// `material_layer_index: vec4<u32>` so the WGSL fragment can address the
/// correct layer when a `texture_2d_array<f32>` collapses N per-material bind
/// groups into one shared bind group with dynamic-offset uniform. Per-material
/// fall-back still allocates a 1-layer array and writes layer index 0.
pub(super) const MATERIAL_UNIFORM_BYTE_LEN: u64 = 96;

/// `min_uniform_buffer_offset_alignment` floor across every wgpu adapter we
/// target. The shared per-batch material uniform buffer pads each entry up to
/// this stride so dynamic-offset binding can point at any layer's slot.
pub(super) const MATERIAL_UNIFORM_ENTRY_STRIDE: u64 = 256;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MaterialUniformUpload {
    pub(super) offset_scale: [f32; 4],
    pub(super) rotation: [f32; 4],
    pub(super) base_color_factor: [f32; 4],
    pub(super) emissive_strength: [f32; 4],
    pub(super) metallic_roughness_alpha: [f32; 4],
    pub(super) material_layer_index: [u32; 4],
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
            material_layer_index: [0, 0, 0, 0],
        }
    }

    /// Plan line 778 commit 2: when the renderer batches N materials into a
    /// shared `texture_2d_array<f32>`, the WGSL sampler call needs to know
    /// which layer to read for this draw. The fall-back per-material path
    /// keeps layer 0 (each material owns a 1-layer array).
    pub(super) fn with_layer_index(mut self, layer: u32) -> Self {
        self.material_layer_index = [layer, 0, 0, 0];
        self
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
            material_layer_index: [0, 0, 0, 0],
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
        // material_layer_index follows the f32 lanes at offset 80.
        for (index, value) in self.material_layer_index.into_iter().enumerate() {
            let byte_offset = 80 + index * 4;
            bytes[byte_offset..byte_offset + 4].copy_from_slice(&value.to_ne_bytes());
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
            96,
            "material uniform must reserve transform, base color, emissive, metallic, \
             roughness, alpha-mask, and material_layer_index lanes (5 vec4<f32> + \
             1 vec4<u32> = 96 bytes)"
        );
    }

    #[test]
    fn material_uniform_upload_encodes_material_layer_index_for_array_batching() {
        let upload = MaterialUniformUpload::identity().with_layer_index(7);
        let bytes = upload.encode();
        // Layer index lives in the trailing vec4<u32> at offset 80. Read back
        // the first lane and confirm it round-trips.
        let lane0 = u32::from_ne_bytes(bytes[80..84].try_into().expect("4 bytes"));
        assert_eq!(lane0, 7);
        assert_eq!(upload.material_layer_index, [7, 0, 0, 0]);
    }

    #[test]
    fn material_uniform_upload_marks_unlit_materials() {
        let material = MaterialDesc::unlit(Color::WHITE);
        let upload = MaterialUniformUpload::from_material(Some(&material), None);

        assert_eq!(upload.metallic_roughness_alpha[3], 1.0);
    }
}
