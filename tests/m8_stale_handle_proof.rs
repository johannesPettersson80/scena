//! Phase 2A — stale-handle proof matrix for the typed asset registry. Closes
//! the state-of-art-threejs-replacement-plan.md line 1252 box that requires a
//! single end-to-end proof that handles built against one `Assets` store fail
//! closed when consumed against another.
//!
//! The individual M8 hot-reload tests already cover descriptor → decoded
//! texture promotion, retained-source scene reload, animation rebinding, and
//! connector stale-handle errors. This file wires the synchronously-created
//! geometry and material handles into one mutually-exclusive proof so a
//! regression in any single try_* path becomes a single test failure instead
//! of needing to inspect the broader hot-reload suites separately.
//!
//! Per the controlling P6 contract, the failure shape must be the typed
//! `AssetError::*HandleNotFound` for each handle kind so user-facing tools can
//! distinguish "wrong asset store" from "missing handle in store".

use std::collections::BTreeMap;
use std::future::ready;
use std::sync::{Arc, Mutex};

use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, Color, GeometryDesc, MaterialDesc, RetainPolicy,
};

#[test]
fn m8_geometry_handle_from_other_store_returns_typed_error() {
    let store_a = Assets::new();
    let store_b = Assets::new();
    let geometry = store_a.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));

    match store_b.try_geometry(geometry) {
        Err(AssetError::GeometryHandleNotFound { .. }) => {}
        other => panic!(
            "geometry handles from a foreign Assets store must surface \
             AssetError::GeometryHandleNotFound, got {other:?}"
        ),
    }
}

#[test]
fn m8_material_handle_from_other_store_returns_typed_error() {
    let store_a = Assets::new();
    let store_b = Assets::new();
    let material = store_a.create_material(MaterialDesc::unlit(Color::WHITE));

    match store_b.try_material(material) {
        Err(AssetError::MaterialHandleNotFound { .. }) => {}
        other => panic!(
            "material handles from a foreign Assets store must surface \
             AssetError::MaterialHandleNotFound, got {other:?}"
        ),
    }
}

#[test]
fn m8_environment_handle_from_other_store_returns_typed_error() {
    // Phase 2A — extends the per-handle stale-handle matrix to the
    // EnvironmentHandle slot. Two independently constructed Assets
    // stores must mint distinct EnvironmentHandle values, and a handle
    // from `store_a.default_environment()` must surface the typed
    // `AssetError::EnvironmentHandleNotFound` when consumed against
    // `store_b`. Closes the environment row in the
    // state-of-art-threejs-replacement-plan.md hot-reload preservation
    // gate (line 1276) for the typed-handle dimension.
    let store_a = Assets::new();
    let store_b = Assets::new();
    let environment = store_a.default_environment();

    match store_b.try_environment(environment) {
        Err(AssetError::EnvironmentHandleNotFound { .. }) => {}
        other => panic!(
            "environment handles from a foreign Assets store must surface \
             AssetError::EnvironmentHandleNotFound, got {other:?}"
        ),
    }
    assert!(
        store_a.contains_environment(environment),
        "environment is live in store_a"
    );
    assert!(
        !store_b.contains_environment(environment),
        "environment is NOT live in store_b — wrong-store case"
    );
}

#[test]
fn m8_assets_store_id_distinguishes_wrong_store_from_stale_handle() {
    // scena-api-ergonomics-reviewer Phase 6 finding F4 closure:
    // The *HandleNotFound diagnostic surface is the same shape for "wrong
    // Assets store" and "handle freed by release_unreferenced", so a
    // beginner needs a programmatic distinguisher. Assets::store_id()
    // labels each Assets instance with a process-unique id, and
    // Assets::contains_<kind>(handle) lets callers check ownership against
    // a specific store before lookup. Together they let beginners
    // distinguish the two failure modes without parsing display text.
    let store_a = Assets::new();
    let store_b = Assets::new();
    assert_ne!(
        store_a.store_id(),
        store_b.store_id(),
        "two independently constructed Assets stores must mint distinct ids",
    );

    let handle = store_a.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    assert!(
        store_a.contains_geometry(handle),
        "handle is live in store_a"
    );
    assert!(
        !store_b.contains_geometry(handle),
        "handle is NOT live in store_b — wrong-store case",
    );

    // Both error paths share the AssetError::GeometryHandleNotFound surface;
    // the predicate-based test above is the path callers should use to
    // distinguish the two failure modes programmatically.
    match store_b.try_geometry(handle) {
        Err(AssetError::GeometryHandleNotFound { .. }) => {}
        other => panic!("wrong-store handle must surface GeometryHandleNotFound: {other:?}"),
    }
}

#[test]
fn m8_assets_store_id_is_stable_across_clone() {
    // A Clone of an Assets instance shares the underlying storage Arc, so
    // Assets::store_id() must remain stable across clones — otherwise a
    // helper that clones the store before resolving handles would surface
    // a misleading "wrong store" reading.
    let store = Assets::new();
    let cloned = store.clone();
    assert_eq!(
        store.store_id(),
        cloned.store_id(),
        "Clone of Assets must preserve store_id since the Arc<Mutex<storage>> is shared",
    );
}

#[test]
fn m8_release_unreferenced_retains_user_created_descriptors_even_when_no_scene_asset_references_them()
 {
    // scena-api-ergonomics-reviewer 4b0e621 finding N2 closure:
    // release_unreferenced is hot-reload-scoped GC, not a generic eviction
    // sweep. User-created descriptors (minted via Assets::create_<kind>)
    // must survive the call so a procedural-scene caller cannot lose
    // handles they still hold. Otherwise the typed *HandleNotFound errors
    // would fire next time the user passes the handle to render-time
    // lookup, contradicting the "no silent fallbacks" review rule.
    let assets = Assets::new();
    let user_geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let user_material = assets.create_material(MaterialDesc::unlit(Color::WHITE));

    let stats = assets.release_unreferenced();
    assert_eq!(
        stats.geometries_evicted, 0,
        "user-created GeometryDesc must survive release_unreferenced; got stats={stats:?}",
    );
    assert_eq!(
        stats.materials_evicted, 0,
        "user-created MaterialDesc must survive release_unreferenced; got stats={stats:?}",
    );
    assert!(
        assets.geometry(user_geometry).is_some(),
        "user-created geometry handle must still resolve after release_unreferenced",
    );
    assert!(
        assets.material(user_material).is_some(),
        "user-created material handle must still resolve after release_unreferenced",
    );
    assert!(
        assets.contains_geometry(user_geometry),
        "contains_geometry must still report ownership after release_unreferenced",
    );
    assert!(
        assets.contains_material(user_material),
        "contains_material must still report ownership after release_unreferenced",
    );
}

#[test]
fn m8_release_unreferenced_with_scene_roots_retains_older_reload_descriptors() {
    let fetcher = MutableMemoryFetcher::new(vec![(
        AssetPath::from("memory://rooted-gc/scene.gltf"),
        coloured_triangle_gltf([1.0, 0.0, 0.0, 1.0]).into_bytes(),
    )]);
    let mut assets = Assets::with_fetcher(fetcher.clone());
    assets.set_retain_policy(RetainPolicy::Always);

    let first = pollster::block_on(assets.load_scene("memory://rooted-gc/scene.gltf"))
        .expect("first scene loads");
    let first_mesh = first.nodes()[0].mesh().expect("first mesh exists");
    let first_geometry = first_mesh.geometry();
    let first_material = first_mesh.material();

    fetcher.insert(
        AssetPath::from("memory://rooted-gc/scene.gltf"),
        coloured_triangle_gltf([0.0, 0.0, 1.0, 1.0]).into_bytes(),
    );
    let reloaded = pollster::block_on(assets.reload_scene(&first)).expect("scene reloads");
    assert_ne!(
        first_material,
        reloaded.nodes()[0]
            .mesh()
            .expect("reloaded mesh exists")
            .material(),
        "reload should create a fresh glTF material handle so the old handle exercises rooted GC",
    );

    let stats = assets.release_unreferenced_with_scene_roots([&first]);
    assert_eq!(
        stats.materials_evicted, 0,
        "old scene material is still live through the explicit scene root: {stats:?}",
    );
    assert!(
        assets.geometry(first_geometry).is_some(),
        "old scene geometry must survive while the caller passes the old SceneAsset as a root",
    );
    assert!(
        assets.material(first_material).is_some(),
        "old scene material must survive while the caller passes the old SceneAsset as a root",
    );
}

#[derive(Clone)]
struct MutableMemoryFetcher {
    files: Arc<Mutex<BTreeMap<AssetPath, Vec<u8>>>>,
}

impl MutableMemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: Arc::new(Mutex::new(files.into_iter().collect())),
        }
    }

    fn insert(&self, path: AssetPath, bytes: Vec<u8>) {
        self.files
            .lock()
            .expect("memory fetcher lock")
            .insert(path, bytes);
    }
}

impl AssetFetcher for MutableMemoryFetcher {
    type Future<'a> = std::future::Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.files
                .lock()
                .expect("memory fetcher lock")
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}

fn coloured_triangle_gltf(base_color: [f32; 4]) -> String {
    format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit"],
            "extensionsRequired": ["KHR_materials_unlit"],
            "materials": [{{
                "pbrMetallicRoughness": {{ "baseColorFactor": [{}, {}, {}, {}] }},
                "extensions": {{ "KHR_materials_unlit": {{}} }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0 }},
                    "indices": 1,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "Root", "mesh": 0 }}],
            "buffers": [{{
                "byteLength": 42,
                "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAAAAAAAAABAAIA"
            }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
                {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#,
        base_color[0], base_color[1], base_color[2], base_color[3]
    )
}
