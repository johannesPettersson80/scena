#![cfg(not(target_arch = "wasm32"))]

use base64::Engine as _;
use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, DirectionalLight, PerspectiveCamera, Renderer,
    Scene, Transform, Vec3,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::Arc;

#[test]
fn quantized_signed_and_unsigned_positions_preserve_vertices_bounds_and_render() {
    for case in [
        QuantizedPositionCase::SignedByte,
        QuantizedPositionCase::SignedByteNormalized,
        QuantizedPositionCase::UnsignedByte,
        QuantizedPositionCase::UnsignedByteNormalized,
        QuantizedPositionCase::SignedShort,
        QuantizedPositionCase::SignedShortNormalized,
        QuantizedPositionCase::UnsignedShort,
        QuantizedPositionCase::UnsignedShortNormalized,
    ] {
        let (document, expected_vertices, expected_min, expected_max) =
            quantized_position_fixture(case);
        let (assets, scene_asset) = load_document("quantized-position.gltf", document)
            .unwrap_or_else(|error| panic!("{case:?} POSITION must load: {error}"));
        let mesh = scene_asset.nodes()[0]
            .mesh()
            .expect("quantized fixture has a mesh");
        let geometry = assets
            .geometry(mesh.geometry())
            .expect("quantized geometry resolves");

        assert_eq!(
            geometry
                .vertices()
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>(),
            expected_vertices,
            "{case:?} POSITION values must retain their unnormalized integer values",
        );
        assert_vec3_near(mesh.bounds().min, expected_min);
        assert_vec3_near(mesh.bounds().max, expected_max);

        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform {
                    translation: Vec3::new(0.0, 0.0, 3.0),
                    ..Transform::default()
                },
            )
            .expect("camera inserts");
        scene
            .instantiate(&scene_asset)
            .expect("quantized fixture instantiates");
        let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("quantized mesh prepares");
        renderer
            .render(&scene, camera)
            .expect("quantized mesh renders");
        assert!(
            renderer
                .frame_rgba8()
                .chunks_exact(4)
                .any(|pixel| pixel[..3] != [0, 0, 0]),
            "{case:?} dequantized node transform must produce visible rendered pixels",
        );
    }
}

#[test]
fn invalid_integer_normal_is_an_error_not_a_default_normal() {
    let error = expect_load_error(
        "invalid-normal.gltf",
        invalid_integer_normal_fixture(),
        "non-normalized unsigned integer NORMAL must be rejected",
    );
    let message = error.to_string();
    assert!(
        message.contains("NORMAL"),
        "error must name NORMAL: {message}"
    );
    assert!(
        message.contains("signed") && message.contains("normalized"),
        "error must explain the valid quantized NORMAL encoding: {message}",
    );
}

#[test]
fn cubic_spline_weights_preserve_target_width_and_tangent_influence() {
    let (assets, scene_asset) = load_document("cubic-weights.gltf", cubic_weights_fixture())
        .expect("valid CUBICSPLINE morph animation loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("valid CUBICSPLINE morph animation instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("morph import frames");
    scene
        .directional_light(DirectionalLight::key_light())
        .transform(Transform::default().rotate_x_deg(-45.0).rotate_y_deg(30.0))
        .add()
        .expect("key light inserts");
    let node = import.node("CubicMorph").expect("morph node resolves");
    let mixer = scene
        .create_animation_mixer(&import, "CubicWeights")
        .expect("mixer creates");

    scene.seek_animation(mixer, 0.0).expect("start samples");
    assert_weights_near(scene.morph_weights(node).unwrap(), &[0.0, 0.0]);
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    renderer.prepare_with_assets(&mut scene, &assets).unwrap();
    renderer.render(&scene, camera).unwrap();
    let start_frame = renderer.frame_rgba8().to_vec();
    scene.seek_animation(mixer, 0.5).expect("midpoint samples");
    assert_weights_near(scene.morph_weights(node).unwrap(), &[0.75, 0.25]);
    scene.seek_animation(mixer, 1.0).expect("endpoint samples");
    assert_weights_near(scene.morph_weights(node).unwrap(), &[1.0, 0.5]);
    renderer.prepare_with_assets(&mut scene, &assets).unwrap();
    renderer.render(&scene, camera).unwrap();
    assert_ne!(
        renderer.frame_rgba8(),
        start_frame.as_slice(),
        "CUBICSPLINE morph deformation must be visible in rendered output",
    );

    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry resolves");
    assert_vec3_near(
        geometry.morph_targets()[0].position_deltas()[2],
        Vec3::new(0.5, 0.0, 1.0),
    );
    assert_vec3_near(geometry.morph_targets()[1].position_deltas()[2], Vec3::ZERO);
    let deformed = geometry
        .morphed_vertices(scene.morph_weights(node).unwrap())
        .expect("morph targets deform");
    assert_vec3_near(deformed[2].position, Vec3::new(0.5, 1.0, 1.0));
}

#[test]
fn multi_primitive_weight_channel_fans_out_to_each_renderable_child() {
    let (assets, scene_asset) =
        load_document("multi-primitive-morph.gltf", multi_primitive_fixture())
            .expect("multi-primitive morph fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("multi-primitive morph fixture instantiates");
    let parent = import.node("MultiMorph").expect("source node resolves");
    let children = scene
        .node(parent)
        .expect("parent exists")
        .children()
        .to_vec();
    assert_eq!(children.len(), 2, "each primitive must be renderable");
    for child in &children {
        assert_weights_near(scene.morph_weights(*child).unwrap(), &[0.0]);
    }

    let mixer = scene
        .create_animation_mixer(&import, "FanOut")
        .expect("fan-out mixer creates");
    scene
        .seek_animation(mixer, 1.0)
        .expect("fan-out seek samples");
    for child in &children {
        assert_weights_near(scene.morph_weights(*child).unwrap(), &[1.0]);
    }
    assert!(
        scene.morph_weights(parent).is_none(),
        "weights belong only to renderable morph children, not the transform parent",
    );
    scene
        .stop_animation(mixer)
        .expect("stop resets fan-out clip");
    scene.play_animation(mixer).expect("fan-out clip plays");
    scene
        .update_animation(mixer, 1.0)
        .expect("fan-out playback advances");
    for child in &children {
        assert_weights_near(scene.morph_weights(*child).unwrap(), &[1.0]);
    }

    let deformed_tips = scene_asset.nodes()[0]
        .meshes()
        .iter()
        .map(|mesh| {
            assets
                .geometry(mesh.geometry())
                .unwrap()
                .morphed_vertices(&[1.0])
                .unwrap()[2]
                .position
        })
        .collect::<Vec<_>>();
    assert_vec3_near(deformed_tips[0], Vec3::new(0.0, 1.5, 0.5));
    assert_vec3_near(deformed_tips[1], Vec3::new(0.5, 1.0, -0.5));
}

#[test]
fn morph_targets_preserve_cardinality_and_optional_normal_tangent_semantics() {
    let (assets, scene_asset) =
        load_document("sparse-morph-semantics.gltf", sparse_morph_fixture())
            .expect("valid sparse morph semantics load");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry resolves");
    let targets = geometry.morph_targets();
    assert_eq!(
        targets.len(),
        2,
        "target order/cardinality must be preserved"
    );
    assert!(
        targets[0]
            .position_deltas()
            .iter()
            .all(|delta| *delta == Vec3::ZERO),
        "normal-only target must receive zero position deltas",
    );
    assert!(targets[0].normal_deltas().is_some());
    assert!(targets[0].tangent_deltas().is_some());
    assert!(
        targets[1]
            .position_deltas()
            .iter()
            .any(|delta| *delta != Vec3::ZERO)
    );
    assert!(targets[1].normal_deltas().is_none());
    assert!(targets[1].tangent_deltas().is_none());

    let morphed = geometry.morphed_vertices(&[1.0, 1.0]).unwrap();
    assert_vec3_near(morphed[0].normal, Vec3::new(1.0, 0.0, 0.0));
    assert_vec3_near(morphed[2].position, Vec3::new(0.0, 1.0, 0.5));
    let tangents = geometry
        .morphed_tangents(&[1.0, 1.0])
        .expect("authored tangent plus tangent morph produces tangents");
    assert_vec3_near(
        Vec3::new(tangents[0][0], tangents[0][1], tangents[0][2]),
        Vec3::new(0.0, 1.0, 0.0),
    );
    assert_eq!(tangents[0][3], -1.0, "tangent handedness is preserved");

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("sparse morph instantiates");
    let node = import
        .node("SparseMorph")
        .expect("sparse morph node resolves");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("sparse morph frames");
    scene
        .directional_light(DirectionalLight::key_light())
        .transform(Transform::default().rotate_x_deg(-45.0).rotate_y_deg(30.0))
        .add()
        .expect("key light inserts");
    scene.set_morph_weights(node, [0.0, 0.0]).unwrap();
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    renderer.prepare_with_assets(&mut scene, &assets).unwrap();
    renderer.render(&scene, camera).unwrap();
    let base_frame = renderer.frame_rgba8().to_vec();
    assert!(
        base_frame
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [0, 0, 0]),
        "normal-mapped sparse morph fixture must render visible pixels",
    );
    scene.set_morph_weights(node, [1.0, 0.0]).unwrap();
    renderer.prepare_with_assets(&mut scene, &assets).unwrap();
    renderer.render(&scene, camera).unwrap();
    assert_ne!(
        renderer.frame_rgba8(),
        base_frame.as_slice(),
        "normal-map lighting must change after normal/tangent morph deltas are applied",
    );
}

#[test]
fn imported_animation_static_policy_accepts_one_key_and_rejects_malformed_clips() {
    let (_, static_asset) = load_document(
        "static-animation.gltf",
        animation_fixture(vec![0.0], OutputShape::Vec3(vec![[2.0, 0.0, 0.0]])),
    )
    .expect("one key at time zero is a valid imported static clip");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&static_asset)
        .expect("static clip instantiates");
    let node = import.node("Animated").unwrap();
    let mixer = scene.create_animation_mixer(&import, "Probe").unwrap();
    scene.seek_animation(mixer, 0.0).unwrap();
    assert_vec3_near(
        scene.node(node).unwrap().transform().translation,
        Vec3::new(2.0, 0.0, 0.0),
    );

    let malformed = [
        ("empty", empty_animation_fixture(), "at least one channel"),
        (
            "nonmonotonic",
            animation_fixture(
                vec![0.0, 1.0, 0.5],
                OutputShape::Vec3(vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]]),
            ),
            "strictly increasing",
        ),
        (
            "duplicate",
            animation_fixture(
                vec![0.0, 0.0],
                OutputShape::Vec3(vec![[0.0; 3], [1.0, 0.0, 0.0]]),
            ),
            "strictly increasing",
        ),
        (
            "nonfinite",
            animation_fixture(
                vec![0.0, f32::NAN],
                OutputShape::Vec3(vec![[0.0; 3], [1.0, 0.0, 0.0]]),
            ),
            "finite",
        ),
        (
            "wrong-count",
            animation_fixture(vec![0.0, 1.0], OutputShape::Vec3(vec![[0.0; 3]])),
            "output length",
        ),
        (
            "wrong-type",
            animation_fixture(
                vec![0.0, 1.0],
                OutputShape::Vec4(vec![[0.0, 0.0, 0.0, 1.0]; 2]),
            ),
            "translation",
        ),
    ];
    for (name, document, expected) in malformed {
        let error = expect_load_error(
            &format!("{name}.gltf"),
            document,
            "malformed imported clip must fail during asset loading",
        );
        assert!(
            error.to_string().contains(expected),
            "{name} error must contain {expected:?}: {error}",
        );
    }
}

#[test]
fn skin_weights_are_validated_and_renormalized_for_all_legal_encodings() {
    for (name, encoding, expected) in [
        (
            "u8-exact",
            SkinWeightEncoding::U8([255, 0, 0, 0]),
            [1.0, 0.0, 0.0, 0.0],
        ),
        (
            "u16-rounding",
            SkinWeightEncoding::U16([32768, 32766, 0, 0]),
            [0.50001526, 0.49998474, 0.0, 0.0],
        ),
        (
            "float-rounding",
            SkinWeightEncoding::F32([0.2, 0.3, 0.5, 0.00001]),
            [0.199998, 0.299997, 0.499995, 0.0000099999],
        ),
    ] {
        let (assets, scene_asset) =
            load_document(&format!("skin-{name}.gltf"), skin_weight_fixture(encoding))
                .unwrap_or_else(|error| panic!("{name} skin weights must load: {error}"));
        let mesh = scene_asset.nodes()[0].mesh().unwrap();
        let geometry = assets.geometry(mesh.geometry()).unwrap();
        let weights = geometry.skin().unwrap().weights()[0];
        assert_weights_near(&weights, &expected);
        assert!((weights.iter().sum::<f32>() - 1.0).abs() <= 1.0e-6);
    }

    for (name, weights, expected) in [
        ("zero", [0.0, 0.0, 0.0, 0.0], "non-zero sum"),
        ("nonfinite", [f32::NAN, 0.0, 0.0, 0.0], "finite"),
        ("negative", [-0.1, 0.6, 0.5, 0.0], "non-negative"),
    ] {
        let error = expect_load_error(
            &format!("skin-{name}.gltf"),
            skin_weight_fixture(SkinWeightEncoding::F32(weights)),
            "invalid skin weights must fail during asset loading",
        );
        assert!(
            error.to_string().contains(expected),
            "{name} error must contain {expected:?}: {error}",
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum QuantizedPositionCase {
    SignedByte,
    SignedByteNormalized,
    UnsignedByte,
    UnsignedByteNormalized,
    SignedShort,
    SignedShortNormalized,
    UnsignedShort,
    UnsignedShortNormalized,
}

fn quantized_position_fixture(case: QuantizedPositionCase) -> (Value, Vec<Vec3>, Vec3, Vec3) {
    let (mut bytes, component_type, normalized, min, max, expected, translation, scale) = match case
    {
        QuantizedPositionCase::SignedByte => (
            vec![254_u8, 254, 0, 2, 254, 0, 0, 2, 0],
            5120,
            false,
            json!([-2, -2, 0]),
            json!([2, 2, 0]),
            vec![
                Vec3::new(-2.0, -2.0, 0.0),
                Vec3::new(2.0, -2.0, 0.0),
                Vec3::new(0.0, 2.0, 0.0),
            ],
            [-0.5, -0.5, 0.0],
            [0.25, 0.25, 0.25],
        ),
        QuantizedPositionCase::SignedByteNormalized => (
            vec![129_u8, 129, 0, 127, 129, 0, 0, 127, 0],
            5120,
            true,
            json!([-127, -127, 0]),
            json!([127, 127, 0]),
            vec![
                Vec3::new(-1.0, -1.0, 0.0),
                Vec3::new(1.0, -1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            [0.0, 0.0, 0.0],
            [0.5, 0.5, 0.5],
        ),
        QuantizedPositionCase::UnsignedByte => (
            vec![0_u8, 0, 0, 4, 0, 0, 0, 4, 0],
            5121,
            false,
            json!([0, 0, 0]),
            json!([4, 4, 0]),
            vec![
                Vec3::ZERO,
                Vec3::new(4.0, 0.0, 0.0),
                Vec3::new(0.0, 4.0, 0.0),
            ],
            [-0.5, -0.5, 0.0],
            [0.25, 0.25, 0.25],
        ),
        QuantizedPositionCase::UnsignedByteNormalized => (
            vec![0_u8, 0, 0, 255, 0, 0, 0, 255, 0],
            5121,
            true,
            json!([0, 0, 0]),
            json!([255, 255, 0]),
            vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            [-0.5, -0.5, 0.0],
            [1.0, 1.0, 1.0],
        ),
        QuantizedPositionCase::SignedShort | QuantizedPositionCase::SignedShortNormalized => {
            let normalized = matches!(case, QuantizedPositionCase::SignedShortNormalized);
            let values = if normalized {
                [-32767_i16, -32767, 0, 32767, -32767, 0, 0, 32767, 0]
            } else {
                [-2_i16, -2, 0, 2, -2, 0, 0, 2, 0]
            };
            let mut bytes = Vec::new();
            for value in values {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            (
                bytes,
                5122,
                normalized,
                if normalized {
                    json!([-32767, -32767, 0])
                } else {
                    json!([-2, -2, 0])
                },
                if normalized {
                    json!([32767, 32767, 0])
                } else {
                    json!([2, 2, 0])
                },
                if normalized {
                    vec![
                        Vec3::new(-1.0, -1.0, 0.0),
                        Vec3::new(1.0, -1.0, 0.0),
                        Vec3::new(0.0, 1.0, 0.0),
                    ]
                } else {
                    vec![
                        Vec3::new(-2.0, -2.0, 0.0),
                        Vec3::new(2.0, -2.0, 0.0),
                        Vec3::new(0.0, 2.0, 0.0),
                    ]
                },
                if normalized {
                    [0.0, 0.0, 0.0]
                } else {
                    [-0.5, -0.5, 0.0]
                },
                if normalized {
                    [0.5, 0.5, 0.5]
                } else {
                    [0.25, 0.25, 0.25]
                },
            )
        }
        QuantizedPositionCase::UnsignedShort => {
            let mut bytes = Vec::new();
            for value in [0_u16, 0, 0, 4, 0, 0, 0, 4, 0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            (
                bytes,
                5123,
                false,
                json!([0, 0, 0]),
                json!([4, 4, 0]),
                vec![
                    Vec3::ZERO,
                    Vec3::new(4.0, 0.0, 0.0),
                    Vec3::new(0.0, 4.0, 0.0),
                ],
                [-0.5, -0.5, 0.0],
                [0.25, 0.25, 0.25],
            )
        }
        QuantizedPositionCase::UnsignedShortNormalized => {
            let mut bytes = Vec::new();
            for value in [0_u16, 0, 0, 65535, 0, 0, 0, 65535, 0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            (
                bytes,
                5123,
                true,
                json!([0, 0, 0]),
                json!([65535, 65535, 0]),
                vec![
                    Vec3::ZERO,
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ],
                [-0.5, -0.5, 0.0],
                [1.0, 1.0, 1.0],
            )
        }
    };
    while bytes.len() % 2 != 0 {
        bytes.push(0);
    }
    let index_offset = bytes.len();
    for index in [0_u16, 1, 2] {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    let byte_length = bytes.len();
    let uri = data_uri(&bytes);
    let expected_min = expected
        .iter()
        .copied()
        .reduce(|left, right| {
            Vec3::new(
                left.x.min(right.x),
                left.y.min(right.y),
                left.z.min(right.z),
            )
        })
        .unwrap();
    let expected_max = expected
        .iter()
        .copied()
        .reduce(|left, right| {
            Vec3::new(
                left.x.max(right.x),
                left.y.max(right.y),
                left.z.max(right.z),
            )
        })
        .unwrap();
    (
        json!({
            "asset": {"version": "2.0"},
            "extensionsUsed": ["KHR_mesh_quantization"],
            "extensionsRequired": ["KHR_mesh_quantization"],
            "nodes": [{
                "name": "Quantized",
                "mesh": 0,
                "translation": translation,
                "scale": scale
            }],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
            "buffers": [{"byteLength": byte_length, "uri": uri}],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": index_offset},
                {"buffer": 0, "byteOffset": index_offset, "byteLength": 6}
            ],
            "accessors": [
                {"bufferView": 0, "componentType": component_type, "normalized": normalized, "count": 3, "type": "VEC3", "min": min, "max": max},
                {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}
            ]
        }),
        expected,
        expected_min,
        expected_max,
    )
}

fn invalid_integer_normal_fixture() -> Value {
    let mut bytes = Vec::new();
    push_f32s(
        &mut bytes,
        &[-0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0],
    );
    let normal_offset = bytes.len();
    bytes.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0, 255]);
    while bytes.len() % 2 != 0 {
        bytes.push(0);
    }
    let index_offset = bytes.len();
    push_u16s(&mut bytes, &[0, 1, 2]);
    json!({
        "asset": {"version": "2.0"},
        "extensionsUsed": ["KHR_mesh_quantization"],
        "extensionsRequired": ["KHR_mesh_quantization"],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2}]}],
        "buffers": [{"byteLength": bytes.len(), "uri": data_uri(&bytes)}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": normal_offset},
            {"buffer": 0, "byteOffset": normal_offset, "byteLength": 9},
            {"buffer": 0, "byteOffset": index_offset, "byteLength": 6}
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.5,-0.5,0.0], "max": [0.5,0.5,0.0]},
            {"bufferView": 1, "componentType": 5121, "count": 3, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}
        ]
    })
}

fn cubic_weights_fixture() -> Value {
    let mut bytes = triangle_positions_indices();
    let morph0 = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 1.0]);
    let morph1 = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let times = bytes.len();
    push_f32s(&mut bytes, &[0.0, 1.0]);
    let outputs = bytes.len();
    push_f32s(
        &mut bytes,
        &[0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.0, 0.0],
    );
    mesh_animation_document(
        bytes,
        vec![
            (0, 36),
            (36, 6),
            (morph0, 36),
            (morph1, 36),
            (times, 8),
            (outputs, 48),
        ],
        json!([
            position_accessor(), index_accessor(),
            {"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3"},
            {"bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC3"},
            {"bufferView": 4, "componentType": 5126, "count": 2, "type": "SCALAR"},
            {"bufferView": 5, "componentType": 5126, "count": 12, "type": "SCALAR"}
        ]),
        json!({"name":"CubicMorph", "mesh":0}),
        json!({"weights":[0.0,0.0], "primitives":[{"attributes":{"POSITION":0},"indices":1,"targets":[{"POSITION":2},{"POSITION":3}]}]}),
        json!({"name":"CubicWeights","samplers":[{"input":4,"output":5,"interpolation":"CUBICSPLINE"}],"channels":[{"sampler":0,"target":{"node":0,"path":"weights"}}]}),
    )
}

fn multi_primitive_fixture() -> Value {
    let mut bytes = triangle_positions_indices();
    let second_positions = bytes.len();
    push_f32s(&mut bytes, &[0.0, -0.5, 0.0, 1.0, -0.5, 0.0, 0.5, 1.0, 0.0]);
    let second_indices = bytes.len();
    push_u16s(&mut bytes, &[0, 1, 2]);
    let first_morph = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5]);
    let second_morph = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.5]);
    let times = bytes.len();
    push_f32s(&mut bytes, &[0.0, 1.0]);
    let weights = bytes.len();
    push_f32s(&mut bytes, &[0.0, 1.0]);
    let views = [
        (0, 36),
        (36, 6),
        (second_positions, 36),
        (second_indices, 6),
        (first_morph, 36),
        (second_morph, 36),
        (times, 8),
        (weights, 8),
    ];
    let buffer_views = views
        .iter()
        .map(|(o, l)| json!({"buffer":0,"byteOffset":o,"byteLength":l}))
        .collect::<Vec<_>>();
    json!({
        "asset":{"version":"2.0"}, "nodes":[{"name":"MultiMorph","mesh":0}],
        "meshes":[{"weights":[0.0],"primitives":[
            {"attributes":{"POSITION":0},"indices":1,"targets":[{"POSITION":4}]},
            {"attributes":{"POSITION":2},"indices":3,"targets":[{"POSITION":5}]}
        ]}],
        "animations":[{"name":"FanOut","samplers":[{"input":6,"output":7}],"channels":[{"sampler":0,"target":{"node":0,"path":"weights"}}]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}], "bufferViews":buffer_views,
        "accessors":[
            position_accessor(), index_accessor(),
            {"bufferView":2,"componentType":5126,"count":3,"type":"VEC3","min":[0.0,-0.5,0.0],"max":[1.0,1.0,0.0]},
            {"bufferView":3,"componentType":5123,"count":3,"type":"SCALAR"},
            {"bufferView":4,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":5,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":6,"componentType":5126,"count":2,"type":"SCALAR"},
            {"bufferView":7,"componentType":5126,"count":2,"type":"SCALAR"}
        ]
    })
}

fn sparse_morph_fixture() -> Value {
    let mut bytes = triangle_positions_indices();
    let normals = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    let tangents = bytes.len();
    push_f32s(
        &mut bytes,
        &[
            1.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, -1.0,
        ],
    );
    let tex_coords = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    let morph_normals = bytes.len();
    push_f32s(
        &mut bytes,
        &[1.0, 0.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0, -1.0],
    );
    let morph_tangents = bytes.len();
    push_f32s(
        &mut bytes,
        &[-1.0, 1.0, 0.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0],
    );
    let morph_positions = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5]);
    let views = [
        (0, 36),
        (36, 6),
        (normals, 36),
        (tangents, 48),
        (tex_coords, 24),
        (morph_normals, 36),
        (morph_tangents, 36),
        (morph_positions, 36),
    ];
    json!({
        "asset":{"version":"2.0"}, "nodes":[{"name":"SparseMorph","mesh":0}],
        "meshes":[{"weights":[0.0,0.0],"primitives":[{"attributes":{"POSITION":0,"NORMAL":2,"TANGENT":3,"TEXCOORD_0":4},"indices":1,"material":0,"targets":[{"NORMAL":5,"TANGENT":6},{"POSITION":7}]}]}],
        "images":[{"uri":normal_map_uri()}],
        "textures":[{"source":0}],
        "materials":[{"normalTexture":{"index":0},"pbrMetallicRoughness":{"baseColorFactor":[1.0,1.0,1.0,1.0],"metallicFactor":0.0,"roughnessFactor":0.5}}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":views.iter().map(|(o,l)|json!({"buffer":0,"byteOffset":o,"byteLength":l})).collect::<Vec<_>>(),
        "accessors":[
            position_accessor(), index_accessor(),
            {"bufferView":2,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":3,"componentType":5126,"count":3,"type":"VEC4"},
            {"bufferView":4,"componentType":5126,"count":3,"type":"VEC2"},
            {"bufferView":5,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":6,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":7,"componentType":5126,"count":3,"type":"VEC3"}
        ]
    })
}

#[derive(Clone)]
enum OutputShape {
    Vec3(Vec<[f32; 3]>),
    Vec4(Vec<[f32; 4]>),
}

fn animation_fixture(times_values: Vec<f32>, output: OutputShape) -> Value {
    let mut bytes = Vec::new();
    push_f32s(&mut bytes, &times_values);
    let output_offset = bytes.len();
    let (kind, count) = match output {
        OutputShape::Vec3(values) => {
            for value in &values {
                push_f32s(&mut bytes, value);
            }
            ("VEC3", values.len())
        }
        OutputShape::Vec4(values) => {
            for value in &values {
                push_f32s(&mut bytes, value);
            }
            ("VEC4", values.len())
        }
    };
    json!({
        "asset":{"version":"2.0"}, "nodes":[{"name":"Animated"}],
        "animations":[{"name":"Probe","samplers":[{"input":0,"output":1}],"channels":[{"sampler":0,"target":{"node":0,"path":"translation"}}]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":output_offset},{"buffer":0,"byteOffset":output_offset,"byteLength":bytes.len()-output_offset}],
        "accessors":[
            {"bufferView":0,"componentType":5126,"count":times_values.len(),"type":"SCALAR"},
            {"bufferView":1,"componentType":5126,"count":count,"type":kind}
        ]
    })
}

fn empty_animation_fixture() -> Value {
    json!({"asset":{"version":"2.0"},"nodes":[{"name":"Animated"}],"animations":[{"name":"Probe","samplers":[],"channels":[]}]})
}

#[derive(Clone, Copy)]
enum SkinWeightEncoding {
    U8([u8; 4]),
    U16([u16; 4]),
    F32([f32; 4]),
}

fn skin_weight_fixture(encoding: SkinWeightEncoding) -> Value {
    let mut bytes = triangle_positions_indices();
    let joints = bytes.len();
    push_u16s(&mut bytes, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let weights = bytes.len();
    let (component_type, normalized, stride) = match encoding {
        SkinWeightEncoding::U8(value) => {
            for _ in 0..3 {
                bytes.extend_from_slice(&value);
            }
            (5121, true, 4)
        }
        SkinWeightEncoding::U16(value) => {
            for _ in 0..3 {
                push_u16s(&mut bytes, &value);
            }
            (5123, true, 8)
        }
        SkinWeightEncoding::F32(value) => {
            for _ in 0..3 {
                push_f32s(&mut bytes, &value);
            }
            (5126, false, 16)
        }
    };
    json!({
        "asset":{"version":"2.0"}, "nodes":[{"name":"Skinned","mesh":0,"skin":0},{"name":"Joint"}],
        "skins":[{"joints":[1]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"JOINTS_0":2,"WEIGHTS_0":3},"indices":1}]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6},{"buffer":0,"byteOffset":joints,"byteLength":24},{"buffer":0,"byteOffset":weights,"byteLength":stride*3}],
        "accessors":[position_accessor(),index_accessor(),
            {"bufferView":2,"componentType":5123,"count":3,"type":"VEC4"},
            {"bufferView":3,"componentType":component_type,"normalized":normalized,"count":3,"type":"VEC4"}
        ]
    })
}

fn mesh_animation_document(
    bytes: Vec<u8>,
    views: Vec<(usize, usize)>,
    accessors: Value,
    node: Value,
    mesh: Value,
    animation: Value,
) -> Value {
    json!({"asset":{"version":"2.0"},"nodes":[node],"meshes":[mesh],"animations":[animation],"buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],"bufferViews":views.iter().map(|(o,l)|json!({"buffer":0,"byteOffset":o,"byteLength":l})).collect::<Vec<_>>(),"accessors":accessors})
}

fn triangle_positions_indices() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_f32s(
        &mut bytes,
        &[-0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 1.0, 0.0],
    );
    push_u16s(&mut bytes, &[0, 1, 2]);
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    bytes
}

fn position_accessor() -> Value {
    json!({"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-0.5,-0.5,0.0],"max":[0.5,1.0,0.0]})
}
fn index_accessor() -> Value {
    json!({"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"})
}
fn push_f32s(bytes: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}
fn push_u16s(bytes: &mut Vec<u8>, values: &[u16]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}
fn data_uri(bytes: &[u8]) -> String {
    format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

fn normal_map_uri() -> String {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(std::io::Cursor::new(&mut bytes), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header writes");
        writer
            .write_image_data(&[255, 128, 255, 255])
            .expect("PNG payload writes");
    }
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes),
    )
}

fn load_document(
    name: &str,
    document: Value,
) -> Result<(Assets<MemoryFetcher>, scena::SceneAsset), AssetError> {
    let path = AssetPath::from(format!("memory://c04/{name}"));
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        path.clone(),
        serde_json::to_vec(&document).unwrap(),
    ));
    let scene_asset = pollster::block_on(assets.load_scene(path.as_str()))?;
    Ok((assets, scene_asset))
}

fn expect_load_error(name: &str, document: Value, context: &str) -> AssetError {
    match load_document(name, document) {
        Ok(_) => panic!("{context}"),
        Err(error) => error,
    }
}

fn assert_vec3_near(actual: Vec3, expected: Vec3) {
    assert!(
        (actual.x - expected.x).abs() <= 1.0e-5
            && (actual.y - expected.y).abs() <= 1.0e-5
            && (actual.z - expected.z).abs() <= 1.0e-5,
        "expected {actual:?} near {expected:?}"
    );
}

fn assert_weights_near(actual: &[f32], expected: &[f32]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "weight widths differ: {actual:?} vs {expected:?}"
    );
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 2.0e-5,
            "weight {index}: expected {actual} near {expected}"
        );
    }
}

#[derive(Clone)]
struct MemoryFetcher {
    sources: Arc<BTreeMap<AssetPath, Vec<u8>>>,
}
impl MemoryFetcher {
    fn new(path: AssetPath, bytes: Vec<u8>) -> Self {
        Self {
            sources: Arc::new(BTreeMap::from([(path, bytes)])),
        }
    }
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
