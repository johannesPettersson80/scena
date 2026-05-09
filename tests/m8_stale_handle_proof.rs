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
