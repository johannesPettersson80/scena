//! Stage C2: KHR_materials_variants parsing now uses the `gltf` crate's
//! `Document::variants()` iterator and `Primitive::mappings()` iterator.

use ::gltf::{Document, Primitive};

use crate::assets::{AssetLoadWarning, AssetPath, MaterialHandle};

pub(super) type RawMaterialVariantMaterialIndices = Vec<Vec<Vec<Option<usize>>>>;

/// Preserve the raw material slot from every variant mapping before the
/// upstream `gltf` facade replaces an out-of-range material with its implicit
/// default material. The nested vectors retain mesh, primitive, and mapping
/// order exactly as authored.
pub(super) fn raw_material_variant_material_indices(
    bytes: &[u8],
) -> RawMaterialVariantMaterialIndices {
    let glb = bytes
        .starts_with(b"glTF")
        .then(|| ::gltf::binary::Glb::from_slice(bytes).ok())
        .flatten();
    let json = glb.as_ref().map_or(bytes, |glb| glb.json.as_ref());
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(json) else {
        return Vec::new();
    };
    value
        .get("meshes")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .map(|mesh| {
            mesh.get("primitives")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .map(|primitive| {
                    primitive
                        .pointer("/extensions/KHR_materials_variants/mappings")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .map(|mapping| {
                            mapping
                                .get("material")
                                .and_then(serde_json::Value::as_u64)
                                .and_then(|index| usize::try_from(index).ok())
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

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
    raw_material_indices: &[Option<usize>],
    path: &AssetPath,
    mesh_index: usize,
    primitive_index: usize,
    load_warnings: &mut Vec<AssetLoadWarning>,
) -> Vec<MaterialVariantBinding> {
    primitive
        .mappings()
        .enumerate()
        .filter_map(|(mapping_index, mapping)| {
            let variants = mapping.variants().to_vec();
            let material_index = raw_material_indices
                .get(mapping_index)
                .copied()
                .flatten()
                .or_else(|| mapping.material().index());
            let Some(material_index) = material_index else {
                load_warnings.push(AssetLoadWarning::InvalidMaterialVariantMapping {
                    path: path.clone(),
                    mesh_index,
                    primitive_index,
                    mapping_index,
                    material_index: None,
                    variant_indices: variants,
                    material_count: materials.len(),
                });
                return None;
            };
            let Some(material) = materials.get(material_index).copied() else {
                load_warnings.push(AssetLoadWarning::InvalidMaterialVariantMapping {
                    path: path.clone(),
                    mesh_index,
                    primitive_index,
                    mapping_index,
                    material_index: Some(material_index),
                    variant_indices: variants,
                    material_count: materials.len(),
                });
                return None;
            };
            Some(MaterialVariantBinding { variants, material })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        parse_material_variant_names, parse_primitive_material_variant_bindings,
        raw_material_variant_material_indices,
    };
    use crate::assets::{AssetLoadWarning, AssetPath, Assets};
    use crate::material::{Color, MaterialDesc};
    use serde_json::json;

    fn document_from_json(value: serde_json::Value) -> ::gltf::Document {
        use crate::assets::AssetPath;
        let bytes = serde_json::to_vec(&value).expect("json serializes");
        let path = AssetPath::from("memory:test");
        let gltf =
            super::super::open_gltf_with_massage(&path, &bytes).expect("json parses as gltf");
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
                "count": 1, "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [0.0, 0.0, 0.0],
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
        let primitive = document
            .meshes()
            .next()
            .unwrap()
            .primitives()
            .next()
            .unwrap();
        let mut warnings = Vec::new();
        let bindings = parse_primitive_material_variant_bindings(
            &primitive,
            &materials,
            &[Some(0), Some(1)],
            &AssetPath::from("memory://valid-variants.gltf"),
            0,
            0,
            &mut warnings,
        );
        assert!(warnings.is_empty());
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].material(), red);
        assert_eq!(bindings[0].variants(), &[0, 2]);
        assert_eq!(bindings[1].material(), blue);
        assert_eq!(bindings[1].variants(), &[1]);
    }

    #[test]
    fn primitive_parser_warns_when_a_variant_material_index_cannot_resolve() {
        let assets = Assets::new();
        let valid_material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let source = json!({
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_variants"],
            "extensions": { "KHR_materials_variants": { "variants": [{ "name": "a" }] } },
            "buffers": [{ "byteLength": 12 }],
            "bufferViews": [{ "buffer": 0, "byteLength": 12 }],
            "accessors": [{
                "bufferView": 0, "componentType": 5126, "count": 1,
                "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [0.0, 0.0, 0.0]
            }],
            "materials": [{}],
            "meshes": [{ "primitives": [{
                "attributes": { "POSITION": 0 },
                "extensions": { "KHR_materials_variants": { "mappings": [
                    { "material": 7, "variants": [0] },
                    { "material": 0, "variants": [0] }
                ] } }
            }] }]
        });
        let raw_indices = raw_material_variant_material_indices(
            &serde_json::to_vec(&source).expect("source JSON serializes"),
        );
        let document = document_from_json(source);
        let primitive = document
            .meshes()
            .next()
            .unwrap()
            .primitives()
            .next()
            .unwrap();
        let mut warnings = Vec::new();
        let bindings = parse_primitive_material_variant_bindings(
            &primitive,
            &[valid_material],
            &raw_indices[0][0],
            &AssetPath::from("memory://invalid-variant.gltf"),
            0,
            0,
            &mut warnings,
        );

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].material(), valid_material);
        assert!(matches!(
            warnings.as_slice(),
            [AssetLoadWarning::InvalidMaterialVariantMapping {
                material_index: Some(7),
                material_count: 1,
                variant_indices,
                ..
            }] if variant_indices == &[0]
        ));
    }
}
