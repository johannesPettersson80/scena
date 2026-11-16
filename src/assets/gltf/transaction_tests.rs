//! Unwind-safety contracts for the glTF import transaction and the asset
//! storage lock. A panic while a load holds the lock must neither poison the
//! store for later callers nor publish any part of the interrupted import.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use base64::Engine as _;
use serde_json::json;

use crate::assets::{Assets, RetainPolicy, TextureCacheUpdatePolicy};
use crate::material::{Color, MaterialDesc};

thread_local! {
    static PANIC_BEFORE_COMMIT: Cell<bool> = const { Cell::new(false) };
}

/// Called by the importer after the whole document was parsed into the
/// transaction copy and before that copy is committed.
pub(super) fn failpoint_before_commit() {
    if PANIC_BEFORE_COMMIT.with(Cell::get) {
        panic!("test failpoint: panic before the glTF import transaction commits");
    }
}

fn with_failpoint<T>(run: impl FnOnce() -> T) -> std::thread::Result<T> {
    PANIC_BEFORE_COMMIT.with(|flag| flag.set(true));
    let result = catch_unwind(AssertUnwindSafe(run));
    PANIC_BEFORE_COMMIT.with(|flag| flag.set(false));
    result
}

fn material_triangle_gltf() -> Vec<u8> {
    let mut buffer = Vec::new();
    for component in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        buffer.extend_from_slice(&component.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
    serde_json::to_vec(&json!({
        "asset": {"version": "2.0"},
        "buffers": [{
            "uri": format!("data:application/octet-stream;base64,{encoded}"),
            "byteLength": buffer.len()
        }],
        "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
        "accessors": [{
            "bufferView": 0,
            "componentType": 5126,
            "count": 3,
            "type": "VEC3",
            "min": [0.0, 0.0, 0.0],
            "max": [1.0, 1.0, 0.0]
        }],
        "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [0.4, 0.5, 0.6, 1.0]}}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "material": 0}]}],
        "nodes": [{"mesh": 0}],
        "scenes": [{"nodes": [0]}],
        "scene": 0
    }))
    .expect("transaction fixture serializes")
}

#[test]
fn panic_while_the_storage_lock_is_held_does_not_poison_later_calls() {
    let assets = Assets::new();
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));

    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let _guard = assets.storage();
        panic!("simulated decoder panic while the asset storage lock is held");
    }));
    assert!(panicked.is_err(), "the simulated panic must unwind");

    assert!(
        assets.contains_material(material),
        "committed descriptors must stay reachable after a panic under the lock",
    );
    let later = assets.create_material(MaterialDesc::unlit(Color::BLACK));
    assert!(assets.contains_material(later));
}

#[test]
fn panic_inside_the_import_transaction_commits_nothing_and_keeps_the_store_usable() {
    let assets = Assets::new();
    let user_material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let bytes = material_triangle_gltf();

    let panicked = with_failpoint(|| {
        pollster::block_on(assets.load_scene_from_bytes("memory://transaction/panic.gltf", &bytes))
    });
    assert!(
        panicked.is_err(),
        "the failpoint must fire inside the import"
    );

    {
        let storage = assets.storage();
        assert_eq!(
            storage.geometries.len(),
            0,
            "an interrupted import must not publish geometry",
        );
        assert_eq!(
            storage.materials.len(),
            1,
            "an interrupted import must not publish materials",
        );
        assert!(
            storage.scene_lookup.is_empty(),
            "an interrupted import must not be cached",
        );
    }
    assert!(assets.contains_material(user_material));

    let scene =
        pollster::block_on(assets.load_scene_from_bytes("memory://transaction/panic.gltf", &bytes))
            .expect("the same store must load the asset once the fault is gone");
    assert_eq!(scene.mesh_count(), 1);
    assert_eq!(assets.storage().geometries.len(), 1);
}

#[test]
fn panic_during_reload_does_not_leak_the_reload_texture_policy_into_the_store() {
    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let bytes = material_triangle_gltf();
    let scene = pollster::block_on(
        assets.load_scene_from_bytes("memory://transaction/reload.gltf", &bytes),
    )
    .expect("fixture loads");

    // The memory path is not fetchable, so reload parses the retained source
    // bytes under the replace-changed-source texture policy.
    let panicked = with_failpoint(|| pollster::block_on(assets.reload_scene(&scene)));
    assert!(
        panicked.is_err(),
        "the failpoint must fire inside the reload"
    );

    assert_eq!(
        assets.storage().texture_cache_update_policy,
        TextureCacheUpdatePolicy::Immutable,
        "an interrupted reload must not leave ordinary loads replacing cached textures",
    );
    pollster::block_on(assets.reload_scene(&scene))
        .expect("reload must succeed once the fault is gone");
}
