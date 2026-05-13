#![cfg(not(target_arch = "wasm32"))]

//! Phase 2B step 3 — runtime KHR_materials_variants flip API. Pins the
//! `Scene::set_active_variant` contract that closes RFC line 1785 and
//! state-of-art-threejs-replacement-plan.md line 1354 ("KHR_materials_variants
//! typed surface remains v1.0 Phase 2B pending"). Asserts that flipping the
//! active variant updates the imported MeshNode's material handle in place,
//! that clearing the active variant restores the primitive's default
//! material, and that an unknown variant name surfaces the typed
//! `LookupError::VariantNotFound` instead of silently no-oping.

use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, LookupError, NodeKind, Scene, SceneImport,
};

#[derive(Clone)]
struct MemoryFetcher {
    files: Arc<Mutex<BTreeMap<AssetPath, Vec<u8>>>>,
}

impl MemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: Arc::new(Mutex::new(files.into_iter().collect())),
        }
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.files
                .lock()
                .expect("memory fetcher mutex should not be poisoned")
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}

fn variants_gltf_with_two_materials() -> Vec<u8> {
    // Triangle vertex buffer: 3 positions (12 bytes each) + 3 indices (u16,
    // 2 bytes each, padded to 4-byte alignment for the accessor). Total
    // 36 bytes positions + 6 bytes indices = 42 bytes.
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_variants", "KHR_materials_unlit"],
            "extensionsRequired": ["KHR_materials_variants"],
            "extensions": {{
                "KHR_materials_variants": {{
                    "variants": [
                        {{ "name": "midnight" }},
                        {{ "name": "noon" }}
                    ]
                }}
            }},
            "materials": [
                {{ "pbrMetallicRoughness": {{ "baseColorFactor": [1.0, 0.0, 0.0, 1.0] }}, "extensions": {{ "KHR_materials_unlit": {{}} }} }},
                {{ "pbrMetallicRoughness": {{ "baseColorFactor": [0.0, 0.0, 1.0, 1.0] }}, "extensions": {{ "KHR_materials_unlit": {{}} }} }},
                {{ "pbrMetallicRoughness": {{ "baseColorFactor": [0.0, 1.0, 0.0, 1.0] }}, "extensions": {{ "KHR_materials_unlit": {{}} }} }}
            ],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0 }},
                    "indices": 1,
                    "material": 0,
                    "extensions": {{
                        "KHR_materials_variants": {{
                            "mappings": [
                                {{ "material": 1, "variants": [0] }},
                                {{ "material": 2, "variants": [1] }}
                            ]
                        }}
                    }}
                }}]
            }}],
            "nodes": [{{ "name": "VariantTriangle", "mesh": 0 }}],
            "scenes": [{{ "nodes": [0] }}],
            "scene": 0,
            "buffers": [{{ "byteLength": 42, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.6, -0.6, 0.0], "max": [0.6, 0.6, 0.0] }},
                {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    gltf.into_bytes()
}

fn load_variants_scene() -> (Assets<MemoryFetcher>, scena::SceneAsset) {
    let path = AssetPath::from("memory://variants/scene.gltf");
    let fetcher = MemoryFetcher::new(vec![(path.clone(), variants_gltf_with_two_materials())]);
    let assets = Assets::with_fetcher(fetcher);
    let scene_asset =
        pollster::block_on(assets.load_scene(path)).expect("variants gltf loads from memory");
    (assets, scene_asset)
}

fn variant_mesh_material(scene: &Scene, import: &SceneImport) -> scena::MaterialHandle {
    // Imported variants attach to mesh nodes recorded under the import.
    // The fixture has exactly one mesh node, so we walk the import's
    // root subtree and return the first MeshNode material.
    for root in import.roots() {
        if let Some(handle) = walk_for_mesh(scene, *root) {
            return handle;
        }
    }
    panic!("scene has no mesh node under variant import");
}

fn walk_for_mesh(scene: &Scene, node_key: scena::NodeKey) -> Option<scena::MaterialHandle> {
    let node = scene.node(node_key)?;
    if let NodeKind::Mesh(mesh) = node.kind() {
        return Some(mesh.material());
    }
    for child in node.children() {
        if let Some(handle) = walk_for_mesh(scene, *child) {
            return Some(handle);
        }
    }
    None
}

#[test]
fn m8_set_active_variant_swaps_imported_mesh_material_handle() {
    // Phase 2B step 3 contract: after `Scene::set_active_variant(import,
    // Some("midnight"))`, the imported MeshNode's material must be the
    // variant-bound MaterialHandle, not the primitive default. Pins the
    // RFC line 1785 v1.0 commitment that the typed Variants map plus
    // active-variant selection on Scene resolves on demand.
    let (_assets, scene_asset) = load_variants_scene();
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("variants scene instantiates");

    assert_eq!(
        import.material_variants(),
        &["midnight".to_string(), "noon".to_string()],
        "SceneImport must surface declared variant names in declaration order",
    );
    assert_eq!(
        import.active_variant(),
        None,
        "no variant active by default"
    );

    let default_material = variant_mesh_material(&scene, &import);

    scene
        .set_active_variant(&import, Some("midnight"))
        .expect("midnight variant resolves to a known name");
    let midnight_material = variant_mesh_material(&scene, &import);
    assert_ne!(
        default_material, midnight_material,
        "midnight variant must swap the MeshNode material to the variant-bound handle",
    );
    assert_eq!(import.active_variant(), Some("midnight".to_string()));

    scene
        .set_active_variant(&import, Some("noon"))
        .expect("noon variant resolves to a known name");
    let noon_material = variant_mesh_material(&scene, &import);
    assert_ne!(
        midnight_material, noon_material,
        "noon variant must swap the MeshNode material to a different bound handle",
    );

    scene
        .set_active_variant(&import, None)
        .expect("clearing the active variant succeeds");
    let cleared_material = variant_mesh_material(&scene, &import);
    assert_eq!(
        cleared_material, default_material,
        "clearing the active variant must restore the primitive's default material",
    );
    assert_eq!(import.active_variant(), None);
}

#[test]
fn m8_set_active_variant_returns_typed_error_for_unknown_name() {
    // Calling `set_active_variant` with a name the asset does not declare
    // must surface `LookupError::VariantNotFound` so callers can list the
    // available names through `import.material_variants()` and recover
    // gracefully. The active-variant slot must remain unchanged.
    let (_assets, scene_asset) = load_variants_scene();
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("variants scene instantiates");

    scene
        .set_active_variant(&import, Some("midnight"))
        .expect("midnight variant resolves before the unknown-name probe");

    match scene.set_active_variant(&import, Some("nonexistent")) {
        Err(LookupError::VariantNotFound { name }) => {
            assert_eq!(name, "nonexistent");
        }
        other => panic!("expected VariantNotFound, got {other:?}"),
    }
    assert_eq!(
        import.active_variant(),
        Some("midnight".to_string()),
        "failed lookup must not stomp the previously-active variant",
    );
}

#[test]
fn m8_active_variant_carries_across_replace_import_when_user_reapplies() {
    // Phase 2B step 3 hot-reload contract: after `Scene::replace_import`,
    // the previous SceneImport's `active_variant()` accessor still
    // reports the name the user had selected (the stale import's
    // variant Mutex is read-only metadata, not gated on live). The
    // user can then call `scene.set_active_variant(&new_import,
    // Some(&previous_name))` to re-apply the same variant on the
    // freshly-instantiated import. Pins the user-facing rebind path
    // so callers can preserve variant selection across hot reload
    // without scena owning the rebind state for them.
    let (_assets, scene_asset) = load_variants_scene();
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("variants scene instantiates");
    scene
        .set_active_variant(&import, Some("midnight"))
        .expect("midnight variant selects before replace_import");

    let replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("replace_import succeeds with the same scene asset");

    let previous_name = import
        .active_variant()
        .expect("the stale import's variant name is still readable after replace_import");
    assert_eq!(previous_name, "midnight");

    assert_eq!(
        replacement.active_variant(),
        None,
        "freshly-instantiated import starts at the default variant slot",
    );
    scene
        .set_active_variant(&replacement, Some(&previous_name))
        .expect("re-applying the previous variant on the replacement import succeeds");
    assert_eq!(
        replacement.active_variant(),
        Some("midnight".to_string()),
        "user-driven variant rebind across replace_import preserves the selection",
    );
}

#[test]
fn m8_imports_without_variants_expose_empty_material_variants() {
    // Assets that do not declare KHR_materials_variants must surface an
    // empty `material_variants()` slice from SceneImport so callers can
    // gate variant UI on `!import.material_variants().is_empty()`.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf"))
            .expect("UnlitTest fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("UnlitTest instantiates");

    assert!(
        import.material_variants().is_empty(),
        "fixtures without KHR_materials_variants must report no variants",
    );
    assert_eq!(import.active_variant(), None);
    assert!(
        scene.set_active_variant(&import, None).is_ok(),
        "clearing variants on an asset without variants must succeed (no-op)",
    );
}
