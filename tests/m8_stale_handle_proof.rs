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

use scena::{AssetError, Assets, Color, GeometryDesc, MaterialDesc};

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
fn m8_release_unreferenced_evicts_dangling_geometry_and_material_descriptors() {
    // scena-gltf-animation-reviewer Phase 6 finding F4 closure:
    // Assets::release_unreferenced() must evict GeometryDesc and
    // MaterialDesc slotmap entries that no cached SceneAsset still
    // references. Without an eviction path long-running hot-reload
    // sessions accumulate dead handles even though scene_lookup keeps
    // only the latest SceneAsset per path.
    let assets = Assets::new();
    let live_geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let live_material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let _stranded_geometry = assets.create_geometry(GeometryDesc::box_xyz(2.0, 2.0, 2.0));
    let _stranded_material = assets.create_material(MaterialDesc::unlit(Color::BLACK));

    // Live handles stay reachable; stranded ones do not.
    assert!(assets.geometry(live_geometry).is_some());
    assert!(assets.material(live_material).is_some());

    let stats = assets.release_unreferenced();
    assert!(
        stats.geometries_evicted >= 2,
        "release_unreferenced must evict every geometry the cache no longer references; got stats={stats:?}",
    );
    assert!(
        stats.materials_evicted >= 2,
        "release_unreferenced must evict every material the cache no longer references; got stats={stats:?}",
    );
    // Live handles also drop because no SceneAsset retains them.
    assert!(
        assets.geometry(live_geometry).is_none(),
        "live geometry not retained by any cached SceneAsset must also evict",
    );
    assert!(
        assets.material(live_material).is_none(),
        "live material not retained by any cached SceneAsset must also evict",
    );
}
