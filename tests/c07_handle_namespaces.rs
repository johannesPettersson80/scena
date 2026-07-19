#![cfg(feature = "scene-host")]

use scena::{
    AssetPath, SceneHostAnimationPlayOptions, SceneHostCore, SceneHostErrorCode, Transform, Vec3,
};

#[test]
fn import_handle_cannot_mutate_the_first_node_slot() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))
    .expect("asset instantiates");
    let root = host.root_handle();
    let before = host
        .scene()
        .node(host.scene().root())
        .expect("root exists")
        .transform();

    let error = host
        .set_transform(import, Transform::at(Vec3::new(9.0, 8.0, 7.0)))
        .expect_err("an import handle must never resolve through the node namespace");
    assert_eq!(error.code(), SceneHostErrorCode::WrongHandleNamespace);

    assert_eq!(
        host.scene()
            .node(host.scene().root())
            .expect("root still exists")
            .transform(),
        before,
        "a wrong-namespace handle must not mutate the colliding node slot"
    );
    assert_ne!(import, root, "public handle kinds need distinct encodings");
}

#[test]
fn every_public_handle_kind_is_distinct_and_wrong_resolvers_are_non_mutating() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let node = host.root_handle();
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("animated asset instantiates");
    let instance = pollster::block_on(host.instantiate_url_instanced(
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        1,
    ))
    .expect("instanced asset creates a root")[0];
    let animation = host
        .play_animation(
            import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions::default(),
        )
        .expect("animation starts");
    let handles = [node, import, instance, animation];
    for (index, handle) in handles.iter().enumerate() {
        assert!(
            handles[index + 1..].iter().all(|other| other != handle),
            "each public handle kind must have a distinct first-slot encoding"
        );
        assert!(
            *handle < (1_u64 << 53),
            "browser-visible handles must remain exact JavaScript integers"
        );
    }

    for wrong in [import, instance, animation] {
        assert_eq!(
            host.node_handle_from_inspection(wrong)
                .expect_err("non-node handle must fail node resolver")
                .code(),
            SceneHostErrorCode::WrongHandleNamespace
        );
    }
    for wrong in [node, instance, animation] {
        assert_eq!(
            host.import_roots(wrong)
                .expect_err("non-import handle must fail import resolver")
                .code(),
            SceneHostErrorCode::WrongHandleNamespace
        );
    }
    for wrong in [node, import, instance] {
        assert_eq!(
            host.pause_animation(wrong)
                .expect_err("non-animation handle must fail animation resolver")
                .code(),
            SceneHostErrorCode::WrongHandleNamespace
        );
    }

    let before = host
        .scene()
        .node(host.scene().root())
        .expect("root exists")
        .transform();
    for wrong in [import, animation] {
        assert_eq!(
            host.set_transform(wrong, Transform::at(Vec3::new(4.0, 5.0, 6.0)))
                .expect_err("wrong handle kind must not mutate a node")
                .code(),
            SceneHostErrorCode::WrongHandleNamespace
        );
    }
    assert_eq!(
        host.scene()
            .node(host.scene().root())
            .expect("root exists")
            .transform(),
        before
    );
}

#[test]
fn every_public_namespace_reuses_slots_without_reviving_stale_handles() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let first_node = host
        .add_empty(None, Transform::IDENTITY, Some("first-node"))
        .expect("node inserts");
    host.remove_node(first_node).expect("node removes");
    let reused_node = host
        .add_empty(None, Transform::IDENTITY, Some("reused-node"))
        .expect("node slot reuses");
    assert_ne!(first_node, reused_node);
    assert_eq!(
        host.set_transform(first_node, Transform::IDENTITY)
            .expect_err("old node generation is stale")
            .code(),
        SceneHostErrorCode::StaleNodeHandle
    );

    let first_instance = pollster::block_on(host.instantiate_url_instanced(
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        1,
    ))
    .expect("instance inserts")[0];
    host.remove_node(first_instance).expect("instance removes");
    let reused_instance = pollster::block_on(host.instantiate_url_instanced(
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        1,
    ))
    .expect("instance slot reuses")[0];
    assert_ne!(first_instance, reused_instance);
    assert_eq!(
        host.set_transform(first_instance, Transform::IDENTITY)
            .expect_err("old instance generation is stale")
            .code(),
        SceneHostErrorCode::StaleNodeHandle
    );

    let first_import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("import inserts");
    let first_animation = host
        .play_animation(
            first_import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions::default(),
        )
        .expect("animation inserts");
    host.remove_import(first_import).expect("import removes");
    assert_eq!(
        host.import_roots(first_import)
            .expect_err("old import generation is stale")
            .code(),
        SceneHostErrorCode::StaleImportHandle
    );
    assert_eq!(
        host.pause_animation(first_animation)
            .expect_err("old animation generation is stale")
            .code(),
        SceneHostErrorCode::StaleAnimationHandle
    );

    let reused_import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("import slot reuses");
    let reused_animation = host
        .play_animation(
            reused_import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions::default(),
        )
        .expect("animation slot reuses");
    assert_ne!(first_import, reused_import);
    assert_ne!(first_animation, reused_animation);
    host.import_roots(reused_import)
        .expect("new import is live");
    host.pause_animation(reused_animation)
        .expect("new animation is live");
}
