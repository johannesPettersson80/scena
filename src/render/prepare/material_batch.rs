//! Phase 1F step 2 (commit 1 of 4): material array-batching plan
//! computation.
//!
//! Determines whether the prepared material slots can share a single
//! `texture_2d_array<f32>` per role (`Capabilities::texture_arrays`
//! batched path) or must keep the per-material 2D bind-group path.
//! All materials must share `(sampler, format, decoded dimensions)`
//! for every populated role; the first incompatibility blocks the
//! batched path. The plan is exposed through `RendererStats` so test
//! harnesses can verify the renderer detects array-batching
//! opportunities.

use crate::assets::{TextureDesc, TextureSamplerDesc, TextureSourceFormat};

use super::resources::PreparedMaterialSlot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialTextureRole {
    BaseColor,
    Normal,
    MetallicRoughness,
    Occlusion,
    Emissive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum MaterialBatchIncompatibility {
    /// The decoded dimensions of two materials' textures for the same
    /// role differ. A single `texture_2d_array` requires every layer
    /// to share width and height.
    DimensionMismatch,
    /// The samplers differ (wrap mode, filter, mipmap policy).
    /// `texture_2d_array` shares one sampler across all layers, so
    /// per-layer sampler differences cannot be expressed.
    SamplerMismatch,
    /// The source formats differ (e.g. one PNG, one KTX2). Array
    /// layers must share the underlying GPU format.
    FormatMismatch,
    /// Browser WebGPU normal-map array batching is disabled until the
    /// M6 browser proof validates tangent-space normal map sampling
    /// through the texture-array path. The per-material path remains
    /// the correctness fallback.
    NormalMapBatchingDeferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialBatchPlan {
    /// True iff every populated role across all materials shares
    /// `(sampler, format, dimensions)`.
    pub batchable: bool,
    /// Number of layers a single `texture_2d_array` per role would
    /// hold. Equals the material slot count when `batchable` is true,
    /// otherwise zero.
    pub layer_count: u32,
    /// First role where compatibility broke down, or `None` when the
    /// plan is batchable. Surfaced through `RendererStats` so a
    /// diagnostic UI can point at the offending texture role.
    pub incompatible_role: Option<MaterialTextureRole>,
    /// First reason an incompatibility kicked in, paired with
    /// `incompatible_role`. Always `None` when the plan is
    /// batchable.
    pub incompatible_reason: Option<MaterialBatchIncompatibility>,
}

impl MaterialBatchPlan {
    pub const fn empty() -> Self {
        Self {
            batchable: true,
            layer_count: 0,
            incompatible_role: None,
            incompatible_reason: None,
        }
    }
}

/// Computes the material array-batching plan for a list of prepared
/// material slots. Walks each role across all materials and records
/// the first incompatibility. Materials with a `None` role slot are
/// considered to share that role's fallback texture and do not break
/// compatibility; only populated roles drive the comparison.
pub(in crate::render) fn compute_material_batch_plan(
    slots: &[PreparedMaterialSlot],
) -> MaterialBatchPlan {
    if slots.is_empty() {
        return MaterialBatchPlan::empty();
    }
    let layer_count = slots.len() as u32;
    for role in [
        MaterialTextureRole::BaseColor,
        MaterialTextureRole::Normal,
        MaterialTextureRole::MetallicRoughness,
        MaterialTextureRole::Occlusion,
        MaterialTextureRole::Emissive,
    ] {
        if role == MaterialTextureRole::Normal && role_is_populated(role, slots) {
            return MaterialBatchPlan {
                batchable: false,
                layer_count: 0,
                incompatible_role: Some(role),
                incompatible_reason: Some(MaterialBatchIncompatibility::NormalMapBatchingDeferred),
            };
        }
        if let Some(reason) = role_compatibility(role, slots) {
            return MaterialBatchPlan {
                batchable: false,
                layer_count: 0,
                incompatible_role: Some(role),
                incompatible_reason: Some(reason),
            };
        }
    }
    MaterialBatchPlan {
        batchable: true,
        layer_count,
        incompatible_role: None,
        incompatible_reason: None,
    }
}

fn role_is_populated(role: MaterialTextureRole, slots: &[PreparedMaterialSlot]) -> bool {
    slots.iter().any(|slot| role_texture(role, slot).is_some())
}

fn role_compatibility(
    role: MaterialTextureRole,
    slots: &[PreparedMaterialSlot],
) -> Option<MaterialBatchIncompatibility> {
    let mut anchor: Option<RoleAnchor> = None;
    for slot in slots {
        let Some(desc) = role_texture(role, slot) else {
            continue;
        };
        let candidate = RoleAnchor::from(desc);
        if let Some(anchor) = anchor.as_ref() {
            if let Some(reason) = anchor.compare(&candidate) {
                return Some(reason);
            }
        } else {
            anchor = Some(candidate);
        }
    }
    None
}

fn role_texture(role: MaterialTextureRole, slot: &PreparedMaterialSlot) -> Option<&TextureDesc> {
    let texture = match role {
        MaterialTextureRole::BaseColor => slot.base_color.as_ref(),
        MaterialTextureRole::Normal => slot.normal.as_ref(),
        MaterialTextureRole::MetallicRoughness => slot.metallic_roughness.as_ref(),
        MaterialTextureRole::Occlusion => slot.occlusion.as_ref(),
        MaterialTextureRole::Emissive => slot.emissive.as_ref(),
    }?;
    Some(&texture.desc)
}

struct RoleAnchor {
    sampler: TextureSamplerDesc,
    format: TextureSourceFormat,
    dimensions: Option<(u32, u32)>,
}

impl RoleAnchor {
    fn from(desc: &TextureDesc) -> Self {
        Self {
            sampler: desc.sampler(),
            format: desc.source_format(),
            dimensions: desc.decoded_dimensions(),
        }
    }

    fn compare(&self, other: &Self) -> Option<MaterialBatchIncompatibility> {
        if self.sampler != other.sampler {
            return Some(MaterialBatchIncompatibility::SamplerMismatch);
        }
        if self.format != other.format {
            return Some(MaterialBatchIncompatibility::FormatMismatch);
        }
        if self.dimensions != other.dimensions {
            return Some(MaterialBatchIncompatibility::DimensionMismatch);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{
        Assets, MaterialHandle, TextureDesc, TextureFilter, TextureSamplerDesc,
        TextureSourceFormat, TextureWrap,
    };
    use crate::material::{Color, MaterialDesc, TextureColorSpace};
    use crate::render::prepare::resources::PreparedMaterialTexture;

    // Minimal valid 1x1 red PNG, base64 decoded once at test setup. The
    // `new_with_bytes` constructor decodes this into 1x1 RGBA pixels so
    // dimension comparisons in the batch plan have something concrete to
    // measure.
    fn one_pixel_png() -> Vec<u8> {
        base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
        )
        .expect("fixture PNG base64 is valid")
    }

    fn texture_desc(sampler: TextureSamplerDesc) -> TextureDesc {
        TextureDesc::new_with_bytes(
            crate::assets::AssetPath::from("memory://material-batch/test.png"),
            TextureColorSpace::Srgb,
            sampler,
            TextureSourceFormat::Png,
            Some(&one_pixel_png()),
        )
        .expect("test PNG decodes")
    }

    fn texture_desc_jpeg(sampler: TextureSamplerDesc) -> TextureDesc {
        // JPEG path stays descriptor-only because the bundled test
        // fixture set does not ship a 1×1 JPEG. The plan compares
        // source_format directly without requiring decoded pixels for
        // that comparison branch.
        TextureDesc::new_with_bytes(
            crate::assets::AssetPath::from("memory://material-batch/test.jpg"),
            TextureColorSpace::Srgb,
            sampler,
            TextureSourceFormat::Jpeg,
            None,
        )
        .expect("descriptor-only JPEG")
    }

    fn material_slot_with_base_color(
        handle: MaterialHandle,
        base_color: TextureDesc,
    ) -> PreparedMaterialSlot {
        PreparedMaterialSlot {
            handle,
            material: MaterialDesc::unlit(Color::WHITE),
            base_color: Some(PreparedMaterialTexture {
                handle: Default::default(),
                desc: base_color,
                transform: None,
            }),
            normal: None,
            metallic_roughness: None,
            occlusion: None,
            emissive: None,
        }
    }

    fn material_slot_with_normal(
        handle: MaterialHandle,
        normal: TextureDesc,
    ) -> PreparedMaterialSlot {
        let mut slot = material_slot_with_base_color(handle, texture_desc(default_sampler()));
        slot.normal = Some(PreparedMaterialTexture {
            handle: Default::default(),
            desc: normal,
            transform: None,
        });
        slot
    }

    fn assets_handle() -> MaterialHandle {
        let assets = Assets::new();
        assets.create_material(MaterialDesc::unlit(Color::WHITE))
    }

    fn default_sampler() -> TextureSamplerDesc {
        TextureSamplerDesc::default()
    }

    fn nearest_sampler() -> TextureSamplerDesc {
        TextureSamplerDesc::new(
            Some(TextureFilter::Nearest),
            Some(TextureFilter::Nearest),
            TextureWrap::ClampToEdge,
            TextureWrap::ClampToEdge,
        )
    }

    #[test]
    fn empty_slot_list_is_batchable_with_zero_layers() {
        let plan = compute_material_batch_plan(&[]);
        assert!(plan.batchable);
        assert_eq!(plan.layer_count, 0);
        assert!(plan.incompatible_role.is_none());
    }

    #[test]
    fn single_material_is_batchable_with_one_layer() {
        let slots = vec![material_slot_with_base_color(
            assets_handle(),
            texture_desc(default_sampler()),
        )];
        let plan = compute_material_batch_plan(&slots);
        assert!(plan.batchable);
        assert_eq!(plan.layer_count, 1);
    }

    #[test]
    fn two_compatible_materials_batch_into_two_layers() {
        let slots = vec![
            material_slot_with_base_color(assets_handle(), texture_desc(default_sampler())),
            material_slot_with_base_color(assets_handle(), texture_desc(default_sampler())),
        ];
        let plan = compute_material_batch_plan(&slots);
        assert!(plan.batchable);
        assert_eq!(plan.layer_count, 2);
    }

    #[test]
    fn normal_mapped_materials_do_not_use_array_batching_until_webgpu_path_is_proven() {
        let slots = vec![
            material_slot_with_normal(assets_handle(), texture_desc(default_sampler())),
            material_slot_with_normal(assets_handle(), texture_desc(default_sampler())),
        ];
        let plan = compute_material_batch_plan(&slots);
        assert!(!plan.batchable);
        assert_eq!(plan.layer_count, 0);
        assert_eq!(plan.incompatible_role, Some(MaterialTextureRole::Normal));
        assert_eq!(
            plan.incompatible_reason,
            Some(MaterialBatchIncompatibility::NormalMapBatchingDeferred),
        );
    }

    #[test]
    fn sampler_mismatch_blocks_batching_with_diagnostic_role() {
        let slots = vec![
            material_slot_with_base_color(assets_handle(), texture_desc(default_sampler())),
            material_slot_with_base_color(assets_handle(), texture_desc(nearest_sampler())),
        ];
        let plan = compute_material_batch_plan(&slots);
        assert!(!plan.batchable);
        assert_eq!(plan.layer_count, 0);
        assert_eq!(plan.incompatible_role, Some(MaterialTextureRole::BaseColor));
        assert_eq!(
            plan.incompatible_reason,
            Some(MaterialBatchIncompatibility::SamplerMismatch),
        );
    }

    #[test]
    fn format_mismatch_blocks_batching_with_diagnostic_role() {
        let slots = vec![
            material_slot_with_base_color(assets_handle(), texture_desc(default_sampler())),
            material_slot_with_base_color(assets_handle(), texture_desc_jpeg(default_sampler())),
        ];
        let plan = compute_material_batch_plan(&slots);
        assert!(!plan.batchable);
        assert_eq!(plan.incompatible_role, Some(MaterialTextureRole::BaseColor));
        assert_eq!(
            plan.incompatible_reason,
            Some(MaterialBatchIncompatibility::FormatMismatch),
        );
    }

    #[test]
    fn normal_map_on_any_material_uses_per_material_path() {
        // One material has a normal map, the other does not. Until
        // browser WebGPU proves normal maps through texture arrays,
        // any populated normal slot keeps the per-material bind-group
        // path.
        let mut left =
            material_slot_with_base_color(assets_handle(), texture_desc(default_sampler()));
        left.normal = Some(PreparedMaterialTexture {
            handle: Default::default(),
            desc: texture_desc(default_sampler()),
            transform: None,
        });
        let right = material_slot_with_base_color(assets_handle(), texture_desc(default_sampler()));
        let plan = compute_material_batch_plan(&[left, right]);
        assert!(!plan.batchable);
        assert_eq!(plan.layer_count, 0);
        assert_eq!(plan.incompatible_role, Some(MaterialTextureRole::Normal));
        assert_eq!(
            plan.incompatible_reason,
            Some(MaterialBatchIncompatibility::NormalMapBatchingDeferred),
        );
    }
}
