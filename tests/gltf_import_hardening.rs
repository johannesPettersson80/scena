#![cfg(not(target_arch = "wasm32"))]

//! Robustness contracts for the glTF importer: every hostile or unusual input
//! in this file must end in a decoded asset or a structured `AssetError`,
//! never in a panic, an abort, or an `Assets` store that stops working.

use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use base64::Engine as _;
use scena::{
    AssetError, AssetFetcher, AssetLoadOptions, AssetPath, Assets, Scene, SceneAsset, Vec3,
};
use serde_json::{Value, json};

const BYTE: u32 = 5120;
const UNSIGNED_BYTE: u32 = 5121;
const SHORT: u32 = 5122;
const UNSIGNED_SHORT: u32 = 5123;
const UNSIGNED_INT: u32 = 5125;
const FLOAT: u32 = 5126;

#[test]
fn normalized_byte_texcoords_load_and_the_asset_store_stays_usable() {
    // The exact shape that used to reach `unreachable!()` inside the glTF
    // reader: KHR_mesh_quantization allows normalized BYTE texture coordinates.
    let mut mesh = TriangleFixture::new();
    let uv = mesh.accessor(&[0, 0, 127, 0, 0, 127], BYTE, "VEC2", 3, Some(true));
    mesh.attribute("TEXCOORD_0", uv);
    mesh.declare_quantization();
    let assets = assets_with([
        ("texcoord-i8.gltf", mesh.document()),
        ("valid.gltf", TriangleFixture::new().document()),
    ]);

    let loaded = catch_unwind(AssertUnwindSafe(|| load(&assets, "texcoord-i8.gltf")))
        .expect("normalized BYTE TEXCOORD_0 must not panic")
        .expect("normalized BYTE TEXCOORD_0 is legal under KHR_mesh_quantization");
    assert_eq!(
        tex_coords(&assets, &loaded),
        vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
    );

    catch_unwind(AssertUnwindSafe(|| load(&assets, "valid.gltf")))
        .expect("the same Assets store must not panic on a later load")
        .expect("the same Assets store must still load a valid asset");
}

#[test]
fn texcoords_decode_every_component_type_the_spec_and_quantization_allow() {
    let cases = [
        ("float", FLOAT, None, f32_bytes(&[0.25, 0.75]), [0.25, 0.75]),
        (
            "byte-normalized",
            BYTE,
            Some(true),
            vec![64, (-128_i8) as u8],
            [64.0 / 127.0, -1.0],
        ),
        (
            "byte",
            BYTE,
            Some(false),
            vec![64, (-3_i8) as u8],
            [64.0, -3.0],
        ),
        (
            "unsigned-byte-normalized",
            UNSIGNED_BYTE,
            Some(true),
            vec![51, 255],
            [0.2, 1.0],
        ),
        (
            "unsigned-byte",
            UNSIGNED_BYTE,
            Some(false),
            vec![51, 255],
            [51.0, 255.0],
        ),
        (
            "short-normalized",
            SHORT,
            Some(true),
            i16_bytes(&[16384, -32768]),
            [16384.0 / 32767.0, -1.0],
        ),
        (
            "short",
            SHORT,
            Some(false),
            i16_bytes(&[300, -7]),
            [300.0, -7.0],
        ),
        (
            "unsigned-short-normalized",
            UNSIGNED_SHORT,
            Some(true),
            u16_bytes(&[13107, 65535]),
            [0.2, 1.0],
        ),
        (
            "unsigned-short",
            UNSIGNED_SHORT,
            Some(false),
            u16_bytes(&[1024, 4096]),
            [1024.0, 4096.0],
        ),
    ];
    for (name, component_type, normalized, element, expected) in cases {
        let mut mesh = TriangleFixture::new();
        let stream = element.repeat(3);
        let uv = mesh.accessor(&stream, component_type, "VEC2", 3, normalized);
        mesh.attribute("TEXCOORD_0", uv);
        mesh.declare_quantization();
        let assets = assets_with([("uv.gltf", mesh.document())]);
        let loaded = catch_unwind(AssertUnwindSafe(|| load(&assets, "uv.gltf")))
            .unwrap_or_else(|_| panic!("{name} TEXCOORD_0 must not panic"))
            .unwrap_or_else(|error| panic!("{name} TEXCOORD_0 must load: {error}"));
        for uv in tex_coords(&assets, &loaded) {
            assert!(
                (uv[0] - expected[0]).abs() <= 1.0e-6 && (uv[1] - expected[1]).abs() <= 1.0e-6,
                "{name} TEXCOORD_0 decoded {uv:?}, expected {expected:?}",
            );
        }
    }
}

#[test]
fn vertex_colors_decode_float_and_normalized_unsigned_integers() {
    let cases = [
        (
            "float-rgb",
            FLOAT,
            None,
            "VEC3",
            f32_bytes(&[0.25, 0.5, 0.75]),
            [0.25, 0.5, 0.75, 1.0],
        ),
        (
            "float-rgba",
            FLOAT,
            None,
            "VEC4",
            f32_bytes(&[0.25, 0.5, 0.75, 0.5]),
            [0.25, 0.5, 0.75, 0.5],
        ),
        (
            "unsigned-byte-rgb",
            UNSIGNED_BYTE,
            Some(true),
            "VEC3",
            vec![255, 51, 0],
            [1.0, 0.2, 0.0, 1.0],
        ),
        (
            "unsigned-byte-rgba",
            UNSIGNED_BYTE,
            Some(true),
            "VEC4",
            vec![255, 51, 0, 102],
            [1.0, 0.2, 0.0, 0.4],
        ),
        (
            "unsigned-short-rgb",
            UNSIGNED_SHORT,
            Some(true),
            "VEC3",
            u16_bytes(&[65535, 13107, 0]),
            [1.0, 0.2, 0.0, 1.0],
        ),
        (
            "unsigned-short-rgba",
            UNSIGNED_SHORT,
            Some(true),
            "VEC4",
            u16_bytes(&[65535, 13107, 0, 26214]),
            [1.0, 0.2, 0.0, 0.4],
        ),
    ];
    for (name, component_type, normalized, kind, element, expected) in cases {
        let mut mesh = TriangleFixture::new();
        let stream = element.repeat(3);
        let color = mesh.accessor(&stream, component_type, kind, 3, normalized);
        mesh.attribute("COLOR_0", color);
        let assets = assets_with([("color.gltf", mesh.document())]);
        let loaded = catch_unwind(AssertUnwindSafe(|| load(&assets, "color.gltf")))
            .unwrap_or_else(|_| panic!("{name} COLOR_0 must not panic"))
            .unwrap_or_else(|error| panic!("{name} COLOR_0 must load: {error}"));
        let geometry = first_geometry(&assets, &loaded);
        assert_eq!(geometry.vertex_colors().len(), 3, "{name}");
        for color in geometry.vertex_colors() {
            let actual = [color.r, color.g, color.b, color.a];
            for (actual, expected) in actual.iter().zip(expected) {
                assert!(
                    (actual - expected).abs() <= 1.0e-6,
                    "{name} COLOR_0 decoded {color:?}, expected {expected:?}",
                );
            }
        }
    }
}

#[test]
fn joints_weights_and_indices_decode_every_legal_integer_width() {
    for (name, joint_type, joint_bytes) in [
        ("unsigned-byte", UNSIGNED_BYTE, vec![1_u8, 0, 0, 0]),
        ("unsigned-short", UNSIGNED_SHORT, u16_bytes(&[1, 0, 0, 0])),
    ] {
        for (weight_name, weight_type, normalized, weight_bytes) in [
            ("float", FLOAT, None, f32_bytes(&[1.0, 0.0, 0.0, 0.0])),
            (
                "unsigned-byte",
                UNSIGNED_BYTE,
                Some(true),
                vec![255_u8, 0, 0, 0],
            ),
            (
                "unsigned-short",
                UNSIGNED_SHORT,
                Some(true),
                u16_bytes(&[65535, 0, 0, 0]),
            ),
        ] {
            let mut mesh = TriangleFixture::new();
            let joints = mesh.accessor(&joint_bytes.repeat(3), joint_type, "VEC4", 3, None);
            let weights =
                mesh.accessor(&weight_bytes.repeat(3), weight_type, "VEC4", 3, normalized);
            mesh.attribute("JOINTS_0", joints);
            mesh.attribute("WEIGHTS_0", weights);
            let assets = assets_with([("skin.gltf", mesh.skinned_document())]);
            let loaded = catch_unwind(AssertUnwindSafe(|| load(&assets, "skin.gltf")))
                .unwrap_or_else(|_| {
                    panic!("{name} joints with {weight_name} weights must not panic")
                })
                .unwrap_or_else(|error| {
                    panic!("{name} joints with {weight_name} weights must load: {error}")
                });
            let geometry = first_geometry(&assets, &loaded);
            let skin = geometry.skin().expect("skin streams are imported");
            assert_eq!(skin.joints(), &[[1, 0, 0, 0]; 3], "{name}/{weight_name}");
            assert_eq!(
                skin.weights(),
                &[[1.0, 0.0, 0.0, 0.0]; 3],
                "{name}/{weight_name}"
            );
        }
    }

    for (name, index_type, index_bytes) in [
        ("unsigned-byte", UNSIGNED_BYTE, vec![2_u8, 1, 0]),
        ("unsigned-short", UNSIGNED_SHORT, u16_bytes(&[2, 1, 0])),
        ("unsigned-int", UNSIGNED_INT, u32_bytes(&[2, 1, 0])),
    ] {
        let mut mesh = TriangleFixture::new();
        let indices = mesh.accessor(&index_bytes, index_type, "SCALAR", 3, None);
        mesh.indices(indices);
        let assets = assets_with([("indices.gltf", mesh.document())]);
        let loaded = catch_unwind(AssertUnwindSafe(|| load(&assets, "indices.gltf")))
            .unwrap_or_else(|_| panic!("{name} indices must not panic"))
            .unwrap_or_else(|error| panic!("{name} indices must load: {error}"));
        assert_eq!(
            first_geometry(&assets, &loaded).indices(),
            &[2, 1, 0],
            "{name}"
        );
    }
}

#[test]
fn unexpected_accessor_layouts_fail_with_errors_that_name_mesh_primitive_and_attribute() {
    struct Case {
        name: &'static str,
        attribute: &'static str,
        component_type: u32,
        kind: &'static str,
        normalized: Option<bool>,
        quantization: bool,
        expected: &'static str,
    }
    let case = |name, attribute, component_type, kind, normalized, quantization, expected| Case {
        name,
        attribute,
        component_type,
        kind,
        normalized,
        quantization,
        expected,
    };
    let cases = [
        case(
            "texcoord-unsigned-int",
            "TEXCOORD_0",
            UNSIGNED_INT,
            "VEC2",
            None,
            true,
            "TEXCOORD_0",
        ),
        case(
            "texcoord-vec3",
            "TEXCOORD_0",
            FLOAT,
            "VEC3",
            None,
            true,
            "TEXCOORD_0",
        ),
        case(
            "texcoord-scalar",
            "TEXCOORD_0",
            FLOAT,
            "SCALAR",
            None,
            true,
            "TEXCOORD_0",
        ),
        case(
            "texcoord-byte-without-extension",
            "TEXCOORD_0",
            BYTE,
            "VEC2",
            Some(true),
            false,
            "KHR_mesh_quantization",
        ),
        case(
            "texcoord-raw-unsigned-short-without-extension",
            "TEXCOORD_0",
            UNSIGNED_SHORT,
            "VEC2",
            Some(false),
            false,
            "KHR_mesh_quantization",
        ),
        case(
            "color-byte",
            "COLOR_0",
            BYTE,
            "VEC4",
            Some(true),
            true,
            "COLOR_0",
        ),
        case(
            "color-short",
            "COLOR_0",
            SHORT,
            "VEC3",
            Some(true),
            true,
            "COLOR_0",
        ),
        case(
            "color-unsigned-int",
            "COLOR_0",
            UNSIGNED_INT,
            "VEC3",
            None,
            true,
            "COLOR_0",
        ),
        case(
            "color-raw-unsigned-byte",
            "COLOR_0",
            UNSIGNED_BYTE,
            "VEC4",
            Some(false),
            true,
            "COLOR_0",
        ),
        case(
            "color-vec2",
            "COLOR_0",
            FLOAT,
            "VEC2",
            None,
            true,
            "COLOR_0",
        ),
        case(
            "joints-byte",
            "JOINTS_0",
            BYTE,
            "VEC4",
            None,
            true,
            "JOINTS_0",
        ),
        case(
            "joints-short",
            "JOINTS_0",
            SHORT,
            "VEC4",
            None,
            true,
            "JOINTS_0",
        ),
        case(
            "joints-unsigned-int",
            "JOINTS_0",
            UNSIGNED_INT,
            "VEC4",
            None,
            true,
            "JOINTS_0",
        ),
        case(
            "joints-float",
            "JOINTS_0",
            FLOAT,
            "VEC4",
            None,
            true,
            "JOINTS_0",
        ),
        case(
            "joints-vec3",
            "JOINTS_0",
            UNSIGNED_SHORT,
            "VEC3",
            None,
            true,
            "JOINTS_0",
        ),
        case(
            "weights-short",
            "WEIGHTS_0",
            SHORT,
            "VEC4",
            Some(true),
            true,
            "WEIGHTS_0",
        ),
        case(
            "weights-unsigned-int",
            "WEIGHTS_0",
            UNSIGNED_INT,
            "VEC4",
            None,
            true,
            "WEIGHTS_0",
        ),
        case(
            "weights-vec3",
            "WEIGHTS_0",
            FLOAT,
            "VEC3",
            None,
            true,
            "WEIGHTS_0",
        ),
        case(
            "indices-byte",
            "indices",
            BYTE,
            "SCALAR",
            None,
            true,
            "indices",
        ),
        case(
            "indices-short",
            "indices",
            SHORT,
            "SCALAR",
            None,
            true,
            "indices",
        ),
        case(
            "indices-float",
            "indices",
            FLOAT,
            "SCALAR",
            None,
            true,
            "indices",
        ),
        case(
            "indices-vec2",
            "indices",
            UNSIGNED_SHORT,
            "VEC2",
            None,
            true,
            "indices",
        ),
    ];
    for case in cases {
        let mut mesh = TriangleFixture::new();
        // Sixteen bytes per element covers every component type and layout
        // above, so the byte range is always valid and only the layout is wrong.
        let stream = mesh.accessor(
            &[0_u8; 48],
            case.component_type,
            case.kind,
            3,
            case.normalized,
        );
        match case.attribute {
            "indices" => mesh.indices(stream),
            "JOINTS_0" => {
                let weights = mesh.accessor(
                    &f32_bytes(&[1.0, 0.0, 0.0, 0.0]).repeat(3),
                    FLOAT,
                    "VEC4",
                    3,
                    None,
                );
                mesh.attribute("JOINTS_0", stream);
                mesh.attribute("WEIGHTS_0", weights);
            }
            "WEIGHTS_0" => {
                let joints = mesh.accessor(&[0_u8; 12], UNSIGNED_BYTE, "VEC4", 3, None);
                mesh.attribute("JOINTS_0", joints);
                mesh.attribute("WEIGHTS_0", stream);
            }
            attribute => mesh.attribute(attribute, stream),
        }
        if case.quantization {
            mesh.declare_quantization();
        }
        let assets = assets_with([
            ("unexpected.gltf", mesh.skinned_document()),
            ("valid.gltf", TriangleFixture::new().document()),
        ]);

        let error = catch_unwind(AssertUnwindSafe(|| load(&assets, "unexpected.gltf")))
            .unwrap_or_else(|_| panic!("{} must not panic", case.name))
            .expect_err("an unexpected accessor layout must be rejected");
        let message = error.to_string();
        assert!(
            matches!(error, AssetError::Parse { .. }),
            "{} must be a structured parse error: {error:?}",
            case.name,
        );
        assert!(
            message.contains("mesh 0") && message.contains("primitive 0"),
            "{} error must name the mesh and primitive: {message}",
            case.name,
        );
        assert!(
            message.contains(case.expected),
            "{} error must name {}: {message}",
            case.name,
            case.expected,
        );
        catch_unwind(AssertUnwindSafe(|| load(&assets, "valid.gltf")))
            .unwrap_or_else(|_| panic!("{}: the store must survive the failed load", case.name))
            .unwrap_or_else(|error| {
                panic!(
                    "{}: the store must still load a valid asset: {error}",
                    case.name
                )
            });
        let leaked = assets.release_unreferenced();
        assert_eq!(
            (leaked.geometries_evicted, leaked.materials_evicted),
            (0, 0),
            "{}: the failed load must not leave descriptors behind",
            case.name,
        );
    }
}

#[test]
fn images_without_a_readable_source_fail_with_a_structured_error() {
    for (name, image, expected) in [
        ("image-without-source", json!({}), "$.images[0]"),
        (
            "image-view-without-mime-type",
            json!({"bufferView": 0}),
            "$.images[0].mimeType",
        ),
    ] {
        let mut document = TriangleFixture::new().document();
        document["images"] = json!([image]);
        document["textures"] = json!([{"source": 0}]);
        document["materials"] = json!([{
            "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}
        }]);
        document["meshes"][0]["primitives"][0]["material"] = json!(0);
        let assets = assets_with([
            ("image.gltf", document),
            ("valid.gltf", TriangleFixture::new().document()),
        ]);

        let error = catch_unwind(AssertUnwindSafe(|| load(&assets, "image.gltf")))
            .unwrap_or_else(|_| panic!("{name} must not panic"))
            .expect_err("an image without a readable source must be rejected");
        assert!(
            matches!(&error, AssetError::Parse { reason, .. } if reason.contains(expected)),
            "{name} must report {expected}: {error:?}",
        );
        load(&assets, "valid.gltf").expect("the store must still load a valid asset");
    }
}

#[test]
fn animation_and_skin_accessors_with_unexpected_layouts_fail_with_structured_errors() {
    // (name, accessor to corrupt, component type, accessor type, expected error text)
    let cases = [
        (
            "rotation-output-unsigned-int",
            "rotation_output",
            UNSIGNED_INT,
            "VEC4",
            "animation 0 channel 1 sampler output",
        ),
        (
            "translation-output-short",
            "translation_output",
            SHORT,
            "VEC3",
            "animation 0 channel 0 sampler output",
        ),
        (
            "keyframe-times-unsigned-short",
            "times",
            UNSIGNED_SHORT,
            "SCALAR",
            "animation 0 channel 0 sampler input",
        ),
        (
            "keyframe-times-vec2",
            "times",
            FLOAT,
            "VEC2",
            "animation 0 channel 0 sampler input",
        ),
        (
            "inverse-bind-matrices-short",
            "inverse_bind_matrices",
            SHORT,
            "MAT4",
            "skin 0 inverseBindMatrices",
        ),
        (
            "inverse-bind-matrices-mat3",
            "inverse_bind_matrices",
            FLOAT,
            "MAT3",
            "skin 0 inverseBindMatrices",
        ),
    ];
    let control = assets_with([("animated.gltf", animated_skin_document(None))]);
    load(&control, "animated.gltf").expect("the uncorrupted animated skin fixture loads");

    for (name, target, component_type, kind, expected) in cases {
        let assets = assets_with([
            (
                "animated.gltf",
                animated_skin_document(Some((target, component_type, kind))),
            ),
            ("valid.gltf", TriangleFixture::new().document()),
        ]);

        let error = catch_unwind(AssertUnwindSafe(|| load(&assets, "animated.gltf")))
            .unwrap_or_else(|_| panic!("{name} must not panic"))
            .expect_err("an unexpected accessor layout must be rejected");
        assert!(
            matches!(&error, AssetError::Parse { reason, .. } if reason.contains(expected)),
            "{name} must name {expected}: {error:?}",
        );
        load(&assets, "valid.gltf").expect("the store must still load a valid asset");
    }
}

#[test]
fn accessors_that_declare_no_elements_fail_with_structured_errors() {
    // The upstream iterator computes `count - 1`, so a stream with `count` 0
    // (or sparse storage with `count` 0) must be rejected before it is built.
    let mut position = TriangleFixture::new();
    position.accessors[0]["count"] = json!(0);

    let mut indices = TriangleFixture::new();
    let empty_indices = indices.accessor(&u16_bytes(&[0, 1, 2]), UNSIGNED_SHORT, "SCALAR", 0, None);
    indices.indices(empty_indices);

    let mut sparse = TriangleFixture::new();
    let uv = sparse.accessor(&f32_bytes(&[0.0; 6]), FLOAT, "VEC2", 3, None);
    let sparse_indices = sparse.view(&[0]);
    let sparse_values = sparse.view(&f32_bytes(&[0.5, 0.5]));
    sparse.accessors[uv]["sparse"] = json!({
        "count": 0,
        "indices": {"bufferView": sparse_indices, "componentType": UNSIGNED_BYTE},
        "values": {"bufferView": sparse_values},
    });
    sparse.attribute("TEXCOORD_0", uv);

    let mut animated = animated_skin_document(None);
    let times = animated["animations"][0]["samplers"][0]["input"]
        .as_u64()
        .expect("fixture sampler names its input accessor") as usize;
    animated["accessors"][times]["count"] = json!(0);

    for (name, document, expected) in [
        (
            "position-count-0",
            position.document(),
            ["POSITION", "count 0"],
        ),
        (
            "indices-count-0",
            indices.document(),
            ["indices", "count 0"],
        ),
        (
            "sparse-count-0",
            sparse.document(),
            ["TEXCOORD_0", "sparse.count 0"],
        ),
        (
            "keyframe-times-count-0",
            animated,
            ["sampler input", "count 0"],
        ),
    ] {
        let assets = assets_with([
            ("empty-stream.gltf", document),
            ("valid.gltf", TriangleFixture::new().document()),
        ]);
        let error = catch_unwind(AssertUnwindSafe(|| load(&assets, "empty-stream.gltf")))
            .unwrap_or_else(|_| panic!("{name} must not panic"))
            .expect_err("a stream without elements must be rejected");
        assert!(
            matches!(
                &error,
                AssetError::Parse { reason, .. }
                    if expected.iter().all(|needle| reason.contains(needle))
            ),
            "{name} must name {expected:?}: {error:?}",
        );
        load(&assets, "valid.gltf").expect("the store must still load a valid asset");
    }
}

#[test]
fn node_chains_deeper_than_the_default_limit_fail_with_a_policy_error() {
    assert_eq!(AssetLoadOptions::default().max_node_depth(), 1_024);
    assert_eq!(AssetLoadOptions::default().max_node_count(), 1_000_000);

    // The reported stack overflow: one chain 200,000 nodes deep.
    let assets = assets_with([
        ("deep-200000.gltf", node_chain_document(200_000)),
        ("deep-1025.gltf", node_chain_document(1_025)),
        ("deep-1024.gltf", node_chain_document(1_024)),
    ]);
    for (name, depth) in [("deep-200000.gltf", 200_000), ("deep-1025.gltf", 1_025)] {
        let error = load(&assets, name).expect_err("a chain past the depth limit must be rejected");
        assert!(
            matches!(
                &error,
                AssetError::PolicyViolation { reason, help, .. }
                    if reason.contains(&depth.to_string())
                        && reason.contains("1024")
                        && help.contains("with_max_node_depth")
            ),
            "{name} must report the measured depth, the limit, and the knob: {error:?}",
        );
    }

    let at_limit = load(&assets, "deep-1024.gltf").expect("a chain at the limit loads");
    assert_eq!(at_limit.node_count(), 1_024);
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&at_limit)
        .expect("a chain at the limit instantiates");
    assert_eq!(import.roots().len(), 1);
    import.node("Leaf").expect("the deepest node is reachable");
}

#[test]
fn a_chain_at_the_default_depth_limit_frames_renders_picks_and_is_removed() {
    // The default limit must be safe for everything a host does after import,
    // not just for the import itself.
    let assets = assets_with([("deep-1024.gltf", node_chain_document(1_024))]);
    let asset = load(&assets, "deep-1024.gltf").expect("a chain at the limit loads");
    let (mut scene, camera) = Scene::with_default_camera().expect("default camera scene builds");
    let import = scene.instantiate(&asset).expect("the chain instantiates");
    let leaf = import.node("Leaf").expect("the deepest node is reachable");

    assert!(scene.world_transform(leaf).is_some());
    let bounds = import
        .bounds_world(&scene)
        .expect("the leaf mesh has world bounds");
    assert_eq!(bounds.max, Vec3::new(1.0, 1.0, 0.0));
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("the imported chain frames");

    let mut renderer = scena::Renderer::headless(32, 32).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("the imported chain prepares");
    renderer
        .render(&scene, camera)
        .expect("the imported chain renders");
    assert!(
        renderer
            .frame_rgba8()
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [0, 0, 0]),
        "the leaf triangle must reach the frame",
    );
    scene
        .pick_with_assets(
            camera,
            scena::CursorPosition::physical(16.0, 16.0),
            scena::Viewport::new(32, 32, 1.0).expect("viewport is valid"),
            &assets,
        )
        .expect("picking walks the imported chain");

    scene
        .remove_import(&import)
        .expect("the imported chain is removed");
    assert!(scene.node(leaf).is_none());
}

#[test]
fn node_count_limit_is_configurable_and_enforced_before_import() {
    let assets = assets_with([("five-nodes.gltf", flat_nodes_document(5))]);
    let path = memory_path("five-nodes.gltf");

    let error = pollster::block_on(assets.load_scene_with_options(
        path.clone(),
        AssetLoadOptions::default().with_max_node_count(4),
    ))
    .expect_err("five nodes exceed a four-node limit");
    assert!(
        matches!(
            &error,
            AssetError::PolicyViolation { reason, help, .. }
                if reason.contains("5 nodes")
                    && reason.contains('4')
                    && help.contains("with_max_node_count")
        ),
        "the error must report the node count, the limit, and the knob: {error:?}",
    );

    let asset = pollster::block_on(assets.load_scene_with_options(
        path.clone(),
        AssetLoadOptions::default().with_max_node_count(5),
    ))
    .expect("five nodes fit a five-node limit");
    assert_eq!(asset.node_count(), 5);

    // A cached scene that was admitted under a looser limit must not satisfy a
    // later, stricter request for the same path.
    pollster::block_on(assets.load_scene(path.clone())).expect("default limits admit five nodes");
    pollster::block_on(
        assets.load_scene_with_options(path, AssetLoadOptions::default().with_max_node_count(4)),
    )
    .expect_err("a cached scene must not bypass a stricter node limit");
}

#[test]
fn a_raised_depth_limit_imports_a_200000_node_chain_without_recursing() {
    let assets = assets_with([("deep.gltf", node_chain_document(200_000))]);
    let asset = pollster::block_on(assets.load_scene_with_options(
        memory_path("deep.gltf"),
        AssetLoadOptions::default().with_max_node_depth(200_000),
    ))
    .expect("a raised depth limit admits the chain");
    assert_eq!(asset.node_count(), 200_000);

    // The leaf mesh is 200,000 levels below the root, so both walks below
    // visit the whole chain.
    let bounds = asset.bounds().expect("the leaf mesh contributes bounds");
    assert_eq!(bounds.min, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(bounds.max, Vec3::new(1.0, 1.0, 0.0));

    let mut scene = Scene::new();
    let import = scene.instantiate(&asset).expect("the chain instantiates");
    assert_eq!(import.roots().len(), 1);
    let leaf = import.node("Leaf").expect("the deepest node is reachable");
    assert!(scene.node(leaf).is_some());
}

#[test]
fn addressable_node_paths_cover_a_deep_named_chain_on_a_small_stack() {
    const DEPTH: usize = 3_000;
    let nodes = (0..DEPTH)
        .map(|index| {
            let mut node = json!({"name": "n"});
            if index + 1 < DEPTH {
                node["children"] = json!([index + 1]);
            }
            node
        })
        .collect::<Vec<_>>();
    let document = json!({
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": nodes,
    });
    let assets = assets_with([("named-chain.gltf", document)]);
    let asset = pollster::block_on(assets.load_scene_with_options(
        memory_path("named-chain.gltf"),
        AssetLoadOptions::default().with_max_node_depth(DEPTH),
    ))
    .expect("the named chain loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&asset)
        .expect("the named chain instantiates");

    // A recursive walk needs a frame per level; 128 KiB cannot hold 3,000 of
    // them, so this only completes when the walk keeps its own work list.
    let paths = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(move || import.addressable_node_paths())
        .expect("small-stack thread starts")
        .join()
        .expect("listing paths must not overflow a small stack");
    // "/" plus one path per named node.
    assert_eq!(paths.len(), DEPTH + 1);
    assert!(paths.contains_key(&"/n".repeat(DEPTH)));
}

#[test]
fn zero_initialised_sparse_accessors_load_within_the_accessor_byte_budget() {
    assert_eq!(AssetLoadOptions::default().max_accessor_bytes(), 1 << 30);
    assert_eq!(
        AssetLoadOptions::default()
            .with_max_accessor_bytes(4_096)
            .max_accessor_bytes(),
        4_096
    );

    // A legitimate use of a sparse accessor without a bufferView: three
    // zero-initialised positions with two of them overridden.
    let assets = assets_with([("sparse.gltf", sparse_position_document(3))]);
    let asset = load(&assets, "sparse.gltf").expect("a small sparse accessor loads");
    let positions = first_geometry(&assets, &asset)
        .vertices()
        .iter()
        .map(|vertex| vertex.position)
        .collect::<Vec<_>>();
    assert_eq!(
        positions,
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0)
        ],
    );

    // The same stream no longer fits a 16-byte budget: three VEC3 floats
    // decode to 36 bytes. The cached scene must not satisfy the stricter
    // request either.
    let error = load_with(
        &assets,
        "sparse.gltf",
        AssetLoadOptions::default().with_max_accessor_bytes(16),
    )
    .expect_err("a budget smaller than the stream must reject it");
    assert!(
        matches!(
            &error,
            AssetError::PolicyViolation { reason, help, .. }
                if reason.contains("mesh 0 primitive 0 POSITION")
                    && reason.contains("36 bytes")
                    && reason.contains("max_accessor_bytes 16")
                    && help.contains("with_max_accessor_bytes")
        ),
        "the error must name the stream, its size, the budget, and the knob: {error:?}",
    );
}

#[test]
fn a_sparse_accessor_with_a_huge_count_fails_before_anything_is_allocated() {
    // Four billion zero-initialised VEC3 floats are 48 GB that no file byte
    // backs: only `count` says so. The budget must stop this at the first read.
    let assets = assets_with([
        ("bomb.gltf", sparse_position_document(4_000_000_000)),
        ("valid.gltf", TriangleFixture::new().document()),
    ]);
    let error = catch_unwind(AssertUnwindSafe(|| load(&assets, "bomb.gltf")))
        .expect("a huge sparse count must not panic")
        .expect_err("a huge sparse count must be rejected");
    assert!(
        matches!(
            &error,
            AssetError::PolicyViolation { reason, .. }
                if reason.contains("POSITION")
                    && reason.contains("48000000000 bytes")
                    && reason.contains("max_accessor_bytes 1073741824")
        ),
        "the error must report the declared size and the default budget: {error:?}",
    );
    load(&assets, "valid.gltf").expect("the store must still load a valid asset");
}

#[test]
fn every_read_of_a_shared_accessor_counts_against_the_budget() {
    // One hundred primitives share a single POSITION/NORMAL pair. Each
    // primitive decodes its own copy, so the budget counts every read.
    let fixture = TriangleFixture::new();
    let mut document = fixture.document();
    document["meshes"][0]["primitives"] = json!(vec![fixture.primitive(); 100]);
    let assets = assets_with([("shared.gltf", document)]);

    let asset = load(&assets, "shared.gltf").expect("the default budget admits all reads");
    assert_eq!(asset.primitive_count(), 100);

    let error = load_with(
        &assets,
        "shared.gltf",
        AssetLoadOptions::default().with_max_accessor_bytes(1_024),
    )
    .expect_err("one kilobyte cannot hold one hundred copies of the streams");
    assert!(
        matches!(
            &error,
            AssetError::PolicyViolation { reason, .. }
                if reason.contains("mesh 0 primitive ") && !reason.contains("primitive 0 ")
        ),
        "the budget must run out on a later primitive, not the first: {error:?}",
    );
}

#[test]
fn derived_vertex_data_counts_against_the_budget() {
    // Morph targets that omit POSITION still need one zero vector per vertex.
    // A thousand of them on a three-vertex primitive are 36,000 bytes that
    // only the JSON array length declares.
    let fixture = TriangleFixture::new();
    let mut targets = fixture.document();
    targets["meshes"][0]["primitives"][0]["targets"] = json!(vec![json!({}); 1_000]);
    targets["meshes"][0]["weights"] = json!(vec![0.0; 1_000]);

    // Flat shading splits every index into its own vertex: 3,000 one-byte
    // indices become 3,000 corners of position, normal, and index.
    let mut unshaded = TriangleFixture::new();
    unshaded.attributes.remove("NORMAL");
    let corner_indices = unshaded.accessor(
        &[0, 1, 2].repeat(1_000),
        UNSIGNED_BYTE,
        "SCALAR",
        3_000,
        None,
    );
    unshaded.indices(corner_indices);

    for (name, document, budget, expected) in [
        ("empty-morph-targets", targets, 4_096, "morph target"),
        (
            "flat-normal-expansion",
            unshaded.document(),
            32_768,
            "flat NORMAL expansion",
        ),
    ] {
        let assets = assets_with([("derived.gltf", document)]);
        load(&assets, "derived.gltf")
            .unwrap_or_else(|error| panic!("{name} must load under the default budget: {error}"));
        let error = load_with(
            &assets,
            "derived.gltf",
            AssetLoadOptions::default().with_max_accessor_bytes(budget),
        )
        .expect_err("derived per-vertex data must count against the budget");
        assert!(
            matches!(
                &error,
                AssetError::PolicyViolation { reason, .. } if reason.contains(expected)
            ),
            "{name} must run out of budget on {expected}: {error:?}",
        );
    }
}

#[test]
fn buffer_views_must_lie_inside_their_buffer() {
    // (name, byteOffset of the NORMAL bufferView)
    for (name, byte_offset) in [
        ("view-past-buffer-end", json!(1_000_000)),
        ("view-range-overflows", json!(u64::MAX - 8)),
    ] {
        let mut document = TriangleFixture::new().document();
        document["bufferViews"][1]["byteOffset"] = byte_offset;
        let assets = assets_with([
            ("view.gltf", document),
            ("valid.gltf", TriangleFixture::new().document()),
        ]);
        let error = catch_unwind(AssertUnwindSafe(|| load(&assets, "view.gltf")))
            .unwrap_or_else(|_| panic!("{name} must not panic"))
            .expect_err("a bufferView outside its buffer must be rejected");
        assert!(
            matches!(
                &error,
                AssetError::Parse { reason, .. } if reason.contains("$.bufferViews[1]")
            ),
            "{name} must name the bufferView: {error:?}",
        );
        load(&assets, "valid.gltf").expect("the store must still load a valid asset");
    }
}

#[test]
fn embedded_buffers_shorter_than_their_byte_length_are_rejected() {
    let fixture = TriangleFixture::new();
    let mut document = fixture.document();
    // Every bufferView still lies inside the bytes that exist; only the
    // buffer's declared length promises more than the data URI holds.
    document["buffers"][0]["byteLength"] = json!(fixture.bytes.len() + 64);
    let assets = assets_with([("short-buffer.gltf", document)]);
    let error = load(&assets, "short-buffer.gltf")
        .expect_err("a buffer shorter than its byteLength must be rejected");
    assert!(
        matches!(
            &error,
            AssetError::Parse { reason, .. }
                if reason.contains("buffer 0") && reason.contains("byteLength")
        ),
        "the error must name the buffer and its declared length: {error:?}",
    );
}

/// A triangle whose POSITION accessor has no bufferView: `count` positions
/// start as zeros and sparse storage overrides elements 1 and 2.
fn sparse_position_document(count: u64) -> Value {
    let mut mesh = TriangleFixture::new();
    let sparse_indices = mesh.view(&[1, 2]);
    let sparse_values = mesh.view(&f32_bytes(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0]));
    mesh.accessors[0] = json!({
        "componentType": FLOAT,
        "count": count,
        "type": "VEC3",
        "min": [0.0, 0.0, 0.0],
        "max": [1.0, 1.0, 0.0],
        "sparse": {
            "count": 2,
            "indices": {"bufferView": sparse_indices, "componentType": UNSIGNED_BYTE},
            "values": {"bufferView": sparse_values},
        },
    });
    mesh.document()
}

/// `depth` nodes in a single parent-to-child chain; the last one carries a
/// triangle mesh and the name `Leaf`.
fn node_chain_document(depth: usize) -> Value {
    let mut document = TriangleFixture::new().document();
    let nodes = (0..depth)
        .map(|index| {
            if index + 1 < depth {
                json!({"children": [index + 1]})
            } else {
                json!({"name": "Leaf", "mesh": 0})
            }
        })
        .collect::<Vec<_>>();
    document["nodes"] = json!(nodes);
    document["scenes"] = json!([{"nodes": [0]}]);
    document
}

/// `count` sibling root nodes without meshes.
fn flat_nodes_document(count: usize) -> Value {
    json!({
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": (0..count).collect::<Vec<_>>()}],
        "nodes": vec![json!({}); count],
    })
}

/// A skinned triangle with one translation and one rotation channel. When
/// `corrupt` names an accessor, only its declared layout changes: every stream
/// is sized so the byte range stays valid for each layout the tests declare.
fn animated_skin_document(corrupt: Option<(&str, u32, &str)>) -> Value {
    let mut mesh = TriangleFixture::new();
    let times = mesh.accessor(&f32_bytes(&[0.0, 1.0, 0.0, 0.0]), FLOAT, "SCALAR", 2, None);
    let translation_output = mesh.accessor(&[0_u8; 32], FLOAT, "VEC3", 2, None);
    let rotation_output = mesh.accessor(
        &f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
        FLOAT,
        "VEC4",
        2,
        None,
    );
    let mut identity = [0.0_f32; 16];
    for diagonal in [0, 5, 10, 15] {
        identity[diagonal] = 1.0;
    }
    let inverse_bind_matrices =
        mesh.accessor(&f32_bytes(&identity).repeat(2), FLOAT, "MAT4", 2, None);
    if let Some((target, component_type, kind)) = corrupt {
        let corrupted = match target {
            "times" => times,
            "translation_output" => translation_output,
            "rotation_output" => rotation_output,
            "inverse_bind_matrices" => inverse_bind_matrices,
            other => panic!("unknown fixture accessor {other}"),
        };
        mesh.accessors[corrupted]["componentType"] = json!(component_type);
        mesh.accessors[corrupted]["type"] = json!(kind);
    }

    let mut document = mesh.skinned_document();
    document["skins"][0]["inverseBindMatrices"] = json!(inverse_bind_matrices);
    document["animations"] = json!([{
        "samplers": [
            {"input": times, "output": translation_output},
            {"input": times, "output": rotation_output}
        ],
        "channels": [
            {"sampler": 0, "target": {"node": 1, "path": "translation"}},
            {"sampler": 1, "target": {"node": 1, "path": "rotation"}}
        ]
    }]);
    document
}

/// One float triangle plus whatever extra accessors a test attaches to it.
struct TriangleFixture {
    bytes: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
    attributes: serde_json::Map<String, Value>,
    indices: Option<usize>,
    extensions_used: Vec<&'static str>,
}

impl TriangleFixture {
    fn new() -> Self {
        let mut fixture = Self {
            bytes: Vec::new(),
            views: Vec::new(),
            accessors: Vec::new(),
            attributes: serde_json::Map::new(),
            indices: None,
            extensions_used: Vec::new(),
        };
        let positions = f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let position = fixture.accessor(&positions, FLOAT, "VEC3", 3, None);
        fixture.accessors[position]["min"] = json!([0.0, 0.0, 0.0]);
        fixture.accessors[position]["max"] = json!([1.0, 1.0, 0.0]);
        let normals = f32_bytes(&[0.0, 0.0, 1.0]).repeat(3);
        let normal = fixture.accessor(&normals, FLOAT, "VEC3", 3, None);
        fixture.attribute("POSITION", position);
        fixture.attribute("NORMAL", normal);
        fixture
    }

    fn view(&mut self, data: &[u8]) -> usize {
        while !self.bytes.len().is_multiple_of(4) {
            self.bytes.push(0);
        }
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": self.bytes.len(),
            "byteLength": data.len(),
        }));
        self.bytes.extend_from_slice(data);
        self.views.len() - 1
    }

    fn accessor(
        &mut self,
        data: &[u8],
        component_type: u32,
        kind: &str,
        count: usize,
        normalized: Option<bool>,
    ) -> usize {
        let view = self.view(data);
        let mut accessor = json!({
            "bufferView": view,
            "componentType": component_type,
            "count": count,
            "type": kind,
        });
        if let Some(normalized) = normalized {
            accessor["normalized"] = json!(normalized);
        }
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }

    fn attribute(&mut self, name: &str, accessor: usize) {
        self.attributes.insert(name.to_owned(), json!(accessor));
    }

    fn indices(&mut self, accessor: usize) {
        self.indices = Some(accessor);
    }

    fn declare_quantization(&mut self) {
        self.extensions_used.push("KHR_mesh_quantization");
    }

    fn primitive(&self) -> Value {
        let mut primitive = json!({"attributes": self.attributes});
        if let Some(indices) = self.indices {
            primitive["indices"] = json!(indices);
        }
        primitive
    }

    fn document(&self) -> Value {
        self.document_with_nodes(json!([{"mesh": 0}]), Value::Null)
    }

    /// Binds the mesh to a two-joint skin so JOINTS_0/WEIGHTS_0 are meaningful.
    fn skinned_document(&self) -> Value {
        self.document_with_nodes(
            json!([{"mesh": 0, "skin": 0}, {"name": "JointA"}, {"name": "JointB"}]),
            json!([{"joints": [1, 2]}]),
        )
    }

    fn document_with_nodes(&self, nodes: Value, skins: Value) -> Value {
        let roots = (0..nodes.as_array().map_or(0, Vec::len)).collect::<Vec<_>>();
        let mut document = json!({
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": roots}],
            "nodes": nodes,
            "meshes": [{"primitives": [self.primitive()]}],
            "accessors": self.accessors,
            "bufferViews": self.views,
            "buffers": [{"byteLength": self.bytes.len(), "uri": data_uri(&self.bytes)}],
        });
        if !skins.is_null() {
            document["skins"] = skins;
        }
        if !self.extensions_used.is_empty() {
            document["extensionsUsed"] = json!(self.extensions_used);
        }
        document
    }
}

fn f32_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn i16_bytes(values: &[i16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn u16_bytes(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn u32_bytes(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn data_uri(bytes: &[u8]) -> String {
    format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

fn memory_path(name: &str) -> AssetPath {
    AssetPath::from(format!("memory://gltf-hardening/{name}"))
}

fn assets_with<const N: usize>(documents: [(&str, Value); N]) -> Assets<MemoryFetcher> {
    Assets::with_fetcher(MemoryFetcher {
        sources: Arc::new(
            documents
                .into_iter()
                .map(|(name, document)| {
                    (
                        memory_path(name),
                        serde_json::to_vec(&document).expect("fixture serializes"),
                    )
                })
                .collect(),
        ),
    })
}

fn load(assets: &Assets<MemoryFetcher>, name: &str) -> Result<SceneAsset, AssetError> {
    pollster::block_on(assets.load_scene(memory_path(name)))
}

fn load_with(
    assets: &Assets<MemoryFetcher>,
    name: &str,
    options: AssetLoadOptions,
) -> Result<SceneAsset, AssetError> {
    pollster::block_on(assets.load_scene_with_options(memory_path(name), options))
}

fn first_geometry(assets: &Assets<MemoryFetcher>, scene_asset: &SceneAsset) -> scena::GeometryDesc {
    let mesh = scene_asset
        .nodes()
        .iter()
        .find_map(|node| node.mesh())
        .expect("fixture has a mesh node");
    assets
        .geometry(mesh.geometry())
        .expect("fixture geometry resolves")
}

fn tex_coords(assets: &Assets<MemoryFetcher>, scene_asset: &SceneAsset) -> Vec<[f32; 2]> {
    first_geometry(assets, scene_asset).tex_coords0().to_vec()
}

#[derive(Clone)]
struct MemoryFetcher {
    sources: Arc<BTreeMap<AssetPath, Vec<u8>>>,
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.sources
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}
