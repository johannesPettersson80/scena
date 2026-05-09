use serde_json::Value as JsonValue;

use crate::assets::MaterialHandle;

/// Phase 2B step 2: a per-primitive entry of the
/// `KHR_materials_variants.mappings[]` array. Maps a list of variant
/// indices (into the top-level variant-name list) to the
/// `MaterialHandle` that should bind when any of those variants is
/// active. The binding survives the asset cache and is consumed by the
/// runtime `Scene::set_active_variant` flip API that lands in step 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialVariantBinding {
    variants: Vec<u32>,
    material: MaterialHandle,
}

impl MaterialVariantBinding {
    pub fn new(variants: Vec<u32>, material: MaterialHandle) -> Self {
        Self { variants, material }
    }

    /// Variant indices that resolve to this binding's material. Indices
    /// reference the top-level `SceneAsset::material_variants` slot
    /// list in declaration order.
    pub fn variants(&self) -> &[u32] {
        &self.variants
    }

    pub fn material(&self) -> MaterialHandle {
        self.material
    }
}

/// Phase 2B step 1: parse the top-level `KHR_materials_variants.variants`
/// array into an ordered list of variant names. Returns an empty vector
/// when the extension is absent or the entries do not declare a `name`.
/// Anonymous entries (no `name` string) are skipped so the returned
/// indices stay in sync with the on-disk variant order — every per-
/// primitive `mappings[].variants[i]` lookup resolves to the same
/// `material_variants[i]` slot.
pub(super) fn parse_material_variant_names(json: &JsonValue) -> Vec<String> {
    let Some(extension) = json
        .get("extensions")
        .and_then(|extensions| extensions.get("KHR_materials_variants"))
    else {
        return Vec::new();
    };
    extension
        .get("variants")
        .and_then(JsonValue::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .get("name")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Phase 2B step 2: parse a primitive's
/// `extensions.KHR_materials_variants.mappings[]` array into a list of
/// `MaterialVariantBinding`s. Each mapping has a `material` index +
/// `variants` index list; this resolves the material index against the
/// already-built `materials` array so the binding stores typed
/// `MaterialHandle`s instead of raw glTF indices. Returns an empty list
/// when the extension is absent on this primitive.
///
/// Mappings whose `material` index falls outside `materials` are
/// dropped — the asset is still loadable (the primitive uses its
/// non-variant default material), but the offending mapping cannot
/// be resolved and a future doctor pass surfaces the diagnostic.
/// Anonymous variant entries in the index list are skipped so the
/// resulting variant index list stays dense.
pub(super) fn parse_primitive_material_variant_bindings(
    primitive: &JsonValue,
    materials: &[MaterialHandle],
) -> Vec<MaterialVariantBinding> {
    let Some(mappings) = primitive
        .get("extensions")
        .and_then(|extensions| extensions.get("KHR_materials_variants"))
        .and_then(|extension| extension.get("mappings"))
        .and_then(JsonValue::as_array)
    else {
        return Vec::new();
    };
    mappings
        .iter()
        .filter_map(|entry| {
            let material_index = entry.get("material").and_then(JsonValue::as_u64)? as usize;
            let material = materials.get(material_index).copied()?;
            let variants = entry
                .get("variants")
                .and_then(JsonValue::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| {
                            value.as_u64().and_then(|index| u32::try_from(index).ok())
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(MaterialVariantBinding { variants, material })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_material_variant_names, parse_primitive_material_variant_bindings};
    use crate::assets::Assets;
    use crate::material::{Color, MaterialDesc};
    use serde_json::json;

    #[test]
    fn parser_returns_empty_when_extension_absent() {
        let asset = json!({
            "asset": { "version": "2.0" },
        });
        assert!(parse_material_variant_names(&asset).is_empty());
    }

    #[test]
    fn parser_reads_variant_names_in_declaration_order() {
        // Phase 2B step 1: KHR_materials_variants.variants is an ordered
        // array of {name: String} objects. The parser must return the
        // names in declaration order so per-primitive
        // `mappings[].variants[i]` lookups in step 2 resolve to
        // `material_variants[i]`.
        let asset = json!({
            "asset": { "version": "2.0" },
            "extensions": {
                "KHR_materials_variants": {
                    "variants": [
                        { "name": "midnight" },
                        { "name": "noon" },
                        { "name": "twilight" },
                    ],
                },
            },
        });
        assert_eq!(
            parse_material_variant_names(&asset),
            vec![
                "midnight".to_string(),
                "noon".to_string(),
                "twilight".to_string(),
            ],
        );
    }

    #[test]
    fn parser_skips_anonymous_entries_so_named_indices_stay_dense() {
        // glTF requires variant entries to declare a `name`, but a future
        // tool that emits anonymous entries should not silently drift the
        // `material_variants[i]` indices the per-primitive bindings will
        // reference. Drop anonymous entries.
        let asset = json!({
            "asset": { "version": "2.0" },
            "extensions": {
                "KHR_materials_variants": {
                    "variants": [
                        { "name": "first" },
                        { "missing": "no_name_here" },
                        { "name": "third" },
                    ],
                },
            },
        });
        assert_eq!(
            parse_material_variant_names(&asset),
            vec!["first".to_string(), "third".to_string()],
        );
    }

    #[test]
    fn parser_returns_empty_for_malformed_variants_array() {
        let asset = json!({
            "asset": { "version": "2.0" },
            "extensions": {
                "KHR_materials_variants": { "variants": "not an array" },
            },
        });
        assert!(parse_material_variant_names(&asset).is_empty());
    }

    #[test]
    fn primitive_parser_returns_empty_when_extension_absent() {
        let primitive = json!({
            "attributes": { "POSITION": 0 },
        });
        let materials = Vec::new();
        assert!(parse_primitive_material_variant_bindings(&primitive, &materials).is_empty());
    }

    #[test]
    fn primitive_parser_resolves_material_indices_to_handles() {
        // Phase 2B step 2: a primitive's KHR_materials_variants.mappings[]
        // entries reference materials by index into the top-level
        // `materials` array. The parser must resolve each index against
        // the already-built `materials: &[MaterialHandle]` slice and
        // surface the typed handle on the returned binding.
        let assets = Assets::new();
        let red =
            assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.0, 0.0)));
        let blue =
            assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.0, 0.0, 1.0)));
        let materials = vec![red, blue];
        let primitive = json!({
            "attributes": { "POSITION": 0 },
            "extensions": {
                "KHR_materials_variants": {
                    "mappings": [
                        { "material": 0, "variants": [0, 2] },
                        { "material": 1, "variants": [1] },
                    ],
                },
            },
        });
        let bindings = parse_primitive_material_variant_bindings(&primitive, &materials);
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].material(), red);
        assert_eq!(bindings[0].variants(), &[0, 2]);
        assert_eq!(bindings[1].material(), blue);
        assert_eq!(bindings[1].variants(), &[1]);
    }

    #[test]
    fn primitive_parser_drops_mappings_with_unresolved_material_index() {
        // A mapping that references a material index outside the
        // already-built materials slice cannot be resolved to a typed
        // handle. Drop the mapping rather than fabricate a fallback —
        // the doctor will surface the upstream extension diagnostic.
        let assets = Assets::new();
        let red =
            assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.0, 0.0)));
        let materials = vec![red];
        let primitive = json!({
            "attributes": { "POSITION": 0 },
            "extensions": {
                "KHR_materials_variants": {
                    "mappings": [
                        { "material": 0, "variants": [0] },
                        { "material": 99, "variants": [1] },
                    ],
                },
            },
        });
        let bindings = parse_primitive_material_variant_bindings(&primitive, &materials);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].material(), red);
    }
}
