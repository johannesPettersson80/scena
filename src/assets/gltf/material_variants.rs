//! Stage C2: KHR_materials_variants parsing now uses the `gltf` crate's
//! `Document::variants()` iterator and `Primitive::mappings()` iterator.

use ::gltf::{Document, Primitive};

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

/// Phase 2B step 1: walk the `Document::variants()` iterator into an
/// ordered list of variant names. Returns an empty vector when the
/// extension is absent. Anonymous entries (no `name`) are skipped so
/// the returned indices stay in sync with the on-disk variant order —
/// every per-primitive `mappings[].variants[i]` lookup resolves to the
/// same `material_variants[i]` slot.
pub(super) fn parse_material_variant_names(document: &Document) -> Vec<String> {
    let Some(variants) = document.variants() else {
        return Vec::new();
    };
    variants.map(|variant| variant.name().to_string()).collect()
}

/// Phase 2B step 2: walk a primitive's
/// `KHR_materials_variants.mappings[]` iterator into typed
/// `MaterialVariantBinding`s. Mappings whose `material` index falls
/// outside `materials` are dropped — the asset is still loadable (the
/// primitive uses its non-variant default material), but the offending
/// mapping cannot be resolved and a future doctor pass surfaces the
/// diagnostic.
pub(super) fn parse_primitive_material_variant_bindings(
    primitive: &Primitive,
    materials: &[MaterialHandle],
) -> Vec<MaterialVariantBinding> {
    primitive
        .mappings()
        .filter_map(|mapping| {
            let material_index = mapping.material().index()?;
            let material = materials.get(material_index).copied()?;
            let variants = mapping.variants().to_vec();
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

    fn document_from_json(value: serde_json::Value) -> ::gltf::Document {
        use crate::assets::AssetPath;
        let bytes = serde_json::to_vec(&value).expect("json serializes");
        let path = AssetPath::from("memory:test");
        let gltf = super::super::open_gltf_with_massage(&path, &bytes)
            .expect("json parses as gltf");
        gltf.document
    }

    #[test]
    fn parser_returns_empty_when_extension_absent() {
        let document = document_from_json(json!({
            "asset": { "version": "2.0" },
        }));
        assert!(parse_material_variant_names(&document).is_empty());
    }

    #[test]
    fn parser_reads_variant_names_in_declaration_order() {
        let document = document_from_json(json!({
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_variants"],
            "extensions": {
                "KHR_materials_variants": {
                    "variants": [
                        { "name": "midnight" },
                        { "name": "noon" },
                        { "name": "twilight" },
                    ],
                },
            },
        }));
        assert_eq!(
            parse_material_variant_names(&document),
            vec![
                "midnight".to_string(),
                "noon".to_string(),
                "twilight".to_string(),
            ],
        );
    }

    #[test]
    fn parser_returns_empty_for_absent_variants_array() {
        let document = document_from_json(json!({
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_variants"],
            "extensions": {
                "KHR_materials_variants": {},
            },
        }));
        assert!(parse_material_variant_names(&document).is_empty());
    }

    #[test]
    fn primitive_parser_resolves_material_indices_to_handles() {
        let assets = Assets::new();
        let red =
            assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.0, 0.0)));
        let blue =
            assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.0, 0.0, 1.0)));
        let materials = vec![red, blue];
        let document = document_from_json(json!({
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_variants"],
            "extensions": {
                "KHR_materials_variants": {
                    "variants": [
                        { "name": "a" },
                        { "name": "b" },
                        { "name": "c" },
                    ],
                },
            },
            "buffers": [{ "byteLength": 12 }],
            "bufferViews": [{ "buffer": 0, "byteLength": 12, "byteOffset": 0 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0, "componentType": 5126,
                "count": 1, "type": "VEC3",
            }],
            "materials": [
                { "pbrMetallicRoughness": { "baseColorFactor": [1.0, 0.0, 0.0, 1.0] }},
                { "pbrMetallicRoughness": { "baseColorFactor": [0.0, 0.0, 1.0, 1.0] }},
            ],
            "meshes": [{
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "extensions": {
                        "KHR_materials_variants": {
                            "mappings": [
                                { "material": 0, "variants": [0, 2] },
                                { "material": 1, "variants": [1] },
                            ],
                        },
                    },
                }],
            }],
        }));
        let primitive = document.meshes().next().unwrap().primitives().next().unwrap();
        let bindings = parse_primitive_material_variant_bindings(&primitive, &materials);
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].material(), red);
        assert_eq!(bindings[0].variants(), &[0, 2]);
        assert_eq!(bindings[1].material(), blue);
        assert_eq!(bindings[1].variants(), &[1]);
    }
}
