#![cfg(not(target_arch = "wasm32"))]

use base64::Engine as _;
use scena::{
    AssetError, AssetFetcher, AssetLoadOptions, AssetLoadReport, AssetPath, Assets,
    DirectionalLight, GeometryTopology, MaterialKind, PerspectiveCamera, Renderer, RetainPolicy,
    Scene, SceneAsset, Transform, Vec3,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::Arc;

#[test]
fn missing_normals_are_computed_per_face_for_indexed_and_nonindexed_meshes() {
    for indexed in [true, false] {
        let (assets, scene_asset) = load_document(
            if indexed {
                "missing-normal-indexed.gltf"
            } else {
                "missing-normal-nonindexed.gltf"
            },
            missing_normal_hard_edge_fixture(indexed, false),
        )
        .unwrap_or_else(|error| panic!("missing NORMAL must compute flat normals: {error}"));
        let mesh = scene_asset.nodes()[0].mesh().expect("fixture mesh exists");
        let geometry = assets.geometry(mesh.geometry()).expect("geometry resolves");

        assert_eq!(geometry.vertices().len(), 6);
        assert_eq!(geometry.indices(), &[0, 1, 2, 3, 4, 5]);
        for normal in geometry.vertices()[..3].iter().map(|vertex| vertex.normal) {
            assert_vec3_near(normal, Vec3::new(0.0, 0.0, 1.0));
        }
        for normal in geometry.vertices()[3..].iter().map(|vertex| vertex.normal) {
            assert_vec3_near(normal, Vec3::new(0.0, 1.0, 0.0));
        }
    }
}

#[test]
fn missing_normals_reject_degenerate_triangles_with_a_precise_error() {
    let error = expect_load_error(
        "missing-normal-degenerate.gltf",
        missing_normal_hard_edge_fixture(false, true),
        "missing NORMAL on a degenerate triangle must fail closed",
    );
    let message = error.to_string();
    assert!(
        message.contains("NORMAL"),
        "error must name NORMAL: {message}"
    );
    assert!(
        message.contains("degenerate triangle 0"),
        "error must identify the irrecoverable face: {message}",
    );
}

#[test]
fn gltf_line_primitives_do_not_require_triangle_normals() {
    let mut fixture = missing_normal_hard_edge_fixture(true, true);
    fixture["meshes"][0]["primitives"][0]["mode"] = json!(1);
    fixture["meshes"][0]["primitives"][0]["material"] = json!(0);
    fixture["meshes"][0]["primitives"][0]["extensions"] = json!({
        "KHR_materials_variants": {
            "mappings": [{ "material": 1, "variants": [0] }]
        }
    });
    fixture["materials"] = json!([
        { "pbrMetallicRoughness": { "baseColorFactor": [1.0, 0.0, 0.0, 1.0] } },
        { "pbrMetallicRoughness": { "baseColorFactor": [0.0, 1.0, 0.0, 1.0] } }
    ]);
    fixture["extensionsUsed"] = json!(["KHR_materials_variants"]);
    fixture["extensions"] = json!({
        "KHR_materials_variants": { "variants": [{ "name": "green" }] }
    });
    let (assets, scene_asset) = load_document("missing-normal-lines.gltf", fixture)
        .expect("glTF LINES use Scena's line topology without triangle normal generation");
    let mesh = scene_asset.nodes()[0].mesh().expect("fixture mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry resolves");

    assert_eq!(geometry.topology(), GeometryTopology::Lines);
    assert_eq!(geometry.indices(), &[0, 1, 2, 0, 3, 1]);
    assert_eq!(
        assets
            .material(mesh.material())
            .expect("line material resolves")
            .kind(),
        MaterialKind::Line,
    );

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("glTF line scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("line asset frames");
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("imported glTF lines prepare as native Scena strokes");
    renderer.render_active(&scene).expect("glTF lines render");
    assert!(
        renderer
            .frame_rgba8()
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8),
        "imported glTF lines must produce visible pixels",
    );
    scene
        .set_active_variant(&import, Some("green"))
        .expect("line material variant activates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("line material variant remains a native Scena stroke");
}

#[test]
fn computed_flat_normals_are_recorded_in_the_asset_load_report() {
    let (_, report) = load_document_report(
        "missing-normal-report.gltf",
        missing_normal_hard_edge_fixture(true, false),
    )
    .expect("missing normals compute with report evidence");
    let warnings = report.to_schema_json()["warnings"]
        .as_array()
        .expect("schema warnings are an array")
        .clone();
    assert!(
        warnings.iter().any(|warning| {
            warning["kind"] == "computed_flat_normals"
                && warning["mesh_index"] == 0
                && warning["primitive_index"] == 0
                && warning["triangle_count"] == 2
        }),
        "computed-normal behavior must be machine-readable: {warnings:?}",
    );
}

#[test]
fn byte_loaded_scene_cache_preserves_semantic_warnings_and_policy_evidence() {
    let path = AssetPath::from("memory://c04/byte-cache-warning.gltf");
    let source = missing_normal_hard_edge_fixture(true, false);
    let bytes = serde_json::to_vec(&source).expect("fixture serializes");

    let disk_assets = Assets::with_fetcher(MemoryFetcher::new(path.clone(), bytes.clone()));
    let disk_report = pollster::block_on(disk_assets.load_scene_with_report(path.as_str()))
        .expect("disk-style first load reports the computed-normal warning");
    assert_eq!(disk_report.warnings().len(), 1);

    let mut byte_assets = Assets::with_fetcher(MemoryFetcher::new(path.clone(), bytes.clone()));
    byte_assets.set_retain_policy(RetainPolicy::Always);
    let byte_scene = pollster::block_on(byte_assets.load_scene_from_bytes(path.clone(), &bytes))
        .expect("byte load succeeds");
    assert_eq!(byte_scene.retained_source_bytes_len(), Some(bytes.len()));

    let cached = pollster::block_on(byte_assets.load_scene_with_report_options(
        path.as_str(),
        AssetLoadOptions::default().with_strict_textures(true),
    ))
    .expect("compatible strict request reuses byte-seeded cache evidence");
    assert!(cached.cache_hit());
    assert_eq!(cached.fetched_bytes(), 0);
    assert_eq!(cached.warnings(), disk_report.warnings());

    let mut changed = source;
    changed["nodes"][0]["name"] = json!("ChangedAfterByteReload");
    let changed_bytes = serde_json::to_vec(&changed).expect("changed fixture serializes");
    pollster::block_on(byte_assets.load_scene_from_bytes(path.clone(), &changed_bytes))
        .expect("changed bytes under the same path replace cached evidence");
    let changed_cached = pollster::block_on(byte_assets.load_scene_with_report(path.as_str()))
        .expect("changed byte load remains cacheable");
    assert!(changed_cached.cache_hit());
    assert_eq!(changed_cached.warnings(), disk_report.warnings());
    assert_eq!(changed_cached.warnings().len(), 1, "warning was duplicated");
    assert_eq!(
        changed_cached.asset().nodes()[0].name(),
        Some("ChangedAfterByteReload")
    );
}

#[test]
fn every_material_texture_slot_rejects_unsupported_texcoord_one_explicitly() {
    for slot in [
        "baseColorTexture",
        "metallicRoughnessTexture",
        "normalTexture",
        "occlusionTexture",
        "emissiveTexture",
        "clearcoatTexture",
    ] {
        let error = expect_load_error(
            &format!("texcoord-one-{slot}.gltf"),
            texture_coordinate_one_fixture(slot),
            "TEXCOORD_1 requests must not silently sample TEXCOORD_0",
        );
        let message = error.to_string();
        assert!(
            message.contains(slot),
            "error must identify texture slot {slot}: {message}",
        );
        assert!(
            message.contains("texCoord 1") && message.contains("TEXCOORD_0"),
            "error must identify requested and supported coordinate sets: {message}",
        );
    }
}

#[test]
fn secondary_skin_influences_select_the_strongest_four_and_report_degradation() {
    let (assets, report) = load_document_report(
        "eight-skin-influences.gltf",
        eight_skin_influences_fixture(),
    )
    .expect("eight source influences load with explicit four-influence degradation");
    let mesh = report.asset().nodes()[0]
        .mesh()
        .expect("fixture mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry resolves");
    let skin = geometry.skin().expect("skin imports");

    assert_eq!(skin.joints()[0], [4, 1, 3, 2]);
    assert_weights_near(
        &skin.weights()[0],
        &[0.470_588_24, 0.235_294_12, 0.176_470_6, 0.117_647_06],
    );
    assert!(
        skin.weights()
            .iter()
            .all(|weights| (weights.iter().sum::<f32>() - 1.0).abs() <= 1.0e-6),
    );

    let warnings = report.to_schema_json()["warnings"]
        .as_array()
        .expect("schema warnings are an array")
        .clone();
    assert!(
        warnings.iter().any(|warning| {
            warning["kind"] == "skin_influences_truncated"
                && warning["source_influences"] == 8
                && warning["retained_influences"] == 4
                && warning["affected_vertices"] == 3
        }),
        "skin influence degradation must be machine-readable: {warnings:?}",
    );
}

#[test]
fn shared_mesh_nodes_apply_distinct_morph_overrides_before_animation() {
    let (assets, scene_asset) = load_document(
        "node-morph-overrides.gltf",
        shared_mesh_node_morph_override_fixture(),
    )
    .expect("shared mesh node-level morph overrides load");
    for (source_index, expected) in [(0, 0.75), (1, 0.25), (2, 0.1)] {
        let source_node = &scene_asset.nodes()[source_index];
        assert_eq!(source_node.meshes().len(), 2);
        for mesh in source_node.meshes() {
            assert_weights_near(mesh.morph_weights(), &[expected]);
        }
    }

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("shared morph mesh instantiates");
    for (name, expected) in [
        ("OverrideA", 0.75),
        ("OverrideB", 0.25),
        ("MeshDefault", 0.1),
    ] {
        let parent = import.node(name).expect("source node resolves");
        let children = scene.node(parent).expect("parent exists").children();
        assert_eq!(children.len(), 2, "multi-primitive node fans out");
        for child in children {
            assert_weights_near(scene.morph_weights(*child).unwrap(), &[expected]);
        }
    }

    let mixer = scene
        .create_animation_mixer(&import, "OverrideAnimation")
        .expect("node-weight animation binds to renderable children");
    scene.seek_animation(mixer, 1.0).expect("animation samples");
    let animated_parent = import.node("OverrideA").unwrap();
    for child in scene.node(animated_parent).unwrap().children() {
        assert_weights_near(scene.morph_weights(*child).unwrap(), &[1.0]);
    }

    let _ = assets;
}

#[test]
fn node_morph_override_width_must_match_every_primitive() {
    let mut document = shared_mesh_node_morph_override_fixture();
    document["nodes"][0]["weights"] = json!([0.75, 0.25]);
    let error = expect_load_error(
        "node-morph-width.gltf",
        document,
        "node morph override cardinality must fail during loading",
    );
    let message = error.to_string();
    assert!(
        message.contains("morph") && message.contains("weight") && message.contains("target"),
        "cardinality error must identify node weights and primitive targets: {message}",
    );
}

#[test]
fn selected_skin_joint_outside_the_bound_skin_fails_predictably() {
    let (assets, scene_asset) = load_document(
        "out-of-range-skin-joint.gltf",
        skin_influences_fixture([4, 5, 6, 99], [0.01, 0.03, 0.06, 0.4]),
    )
    .expect("joint index is validated against the node's bound skin at prepare time");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("skin node hierarchy instantiates");
    scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    let error = renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect_err("selected joint 99 must exceed the eight-joint binding");
    let message = error.to_string();
    assert!(
        message.contains("99") && message.contains("8"),
        "error must identify joint index and bound joint count: {message}",
    );
}

#[test]
fn khronos_simple_skin_uses_computed_normals_through_skinning_and_cpu_render() {
    let assets = Assets::new();
    let report = pollster::block_on(
        assets.load_scene_with_report("tests/assets/gltf/khronos/SimpleSkin/SimpleSkin.gltf"),
    )
    .expect("real Khronos SimpleSkin loads");
    assert!(
        report
            .warnings()
            .iter()
            .any(|warning| matches!(warning, scena::AssetLoadWarning::ComputedFlatNormals { .. })),
        "SimpleSkin omits NORMAL and must report computed flat shading",
    );
    let scene_asset = report.asset();
    let mesh = scene_asset.nodes()[0]
        .mesh()
        .expect("SimpleSkin mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry resolves");
    assert_eq!(
        geometry.vertices().len(),
        geometry.indices().len(),
        "missing-normal indexed geometry must be split per triangle corner",
    );
    assert!(
        geometry.skin().is_some(),
        "skin streams survive vertex splitting"
    );

    let mut scene = Scene::new();
    let import = scene
        .instantiate(scene_asset)
        .expect("SimpleSkin instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene.frame_import(camera, &import).expect("asset frames");
    scene
        .directional_light(DirectionalLight::key_light())
        .add()
        .expect("key light inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("computed-normal skin prepares");
    renderer.render_active(&scene).expect("SimpleSkin renders");
    assert!(
        renderer
            .frame_rgba8()
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8)
            .count()
            > 32,
        "real computed-normal skinned asset must render nonblank",
    );
}

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
fn quantized_tangents_and_morph_deltas_decode_without_panics_or_zeroing() {
    let (assets, scene_asset) = load_document(
        "quantized-tangent-morph.gltf",
        quantized_tangent_morph_fixture(),
    )
    .expect("KHR_mesh_quantization tangent and morph streams load");
    let mesh = scene_asset.nodes()[0].mesh().expect("fixture mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry resolves");

    let tangents = geometry.tangents().expect("quantized base tangents decode");
    assert_eq!(tangents.len(), 3);
    assert_vec3_near(
        Vec3::new(tangents[0][0], tangents[0][1], tangents[0][2]),
        Vec3::new(1.0, 0.0, 0.0),
    );
    assert_eq!(tangents[0][3], -1.0);

    let target = &geometry.morph_targets()[0];
    assert_vec3_near(target.position_deltas()[2], Vec3::new(0.0, 0.0, 2.0));
    assert_vec3_near(
        target.normal_deltas().expect("normal deltas decode")[0],
        Vec3::new(1.0, 0.0, -1.0),
    );
    assert_vec3_near(
        target.tangent_deltas().expect("tangent deltas decode")[0],
        Vec3::new(-1.0, 1.0, 0.0),
    );
}

#[test]
fn quantized_tangent_and_morph_accessors_require_extension_declaration() {
    let mut fixture = quantized_tangent_morph_fixture();
    fixture
        .as_object_mut()
        .expect("fixture root is an object")
        .remove("extensionsUsed");
    fixture
        .as_object_mut()
        .expect("fixture root is an object")
        .remove("extensionsRequired");

    let error = expect_load_error(
        "undeclared-quantized-tangent-morph.gltf",
        fixture,
        "integer tangent/morph streams require KHR_mesh_quantization",
    );
    let message = error.to_string();
    assert!(
        message.contains("KHR_mesh_quantization") && message.contains("TANGENT"),
        "error must name the missing extension declaration and semantic: {message}"
    );
}

#[test]
fn non_finite_tangent_and_morph_values_fail_closed() {
    for (label, fixture, semantic) in [
        (
            "non-finite-tangent.gltf",
            non_finite_mesh_stream_fixture(false),
            "TANGENT",
        ),
        (
            "non-finite-morph.gltf",
            non_finite_mesh_stream_fixture(true),
            "morph target 0 POSITION",
        ),
    ] {
        let error = expect_load_error(label, fixture, "non-finite mesh streams must fail");
        let message = error.to_string();
        assert!(
            message.contains(semantic) && message.contains("non-finite"),
            "error must identify {semantic} and the finite-value contract: {message}"
        );
    }
}

#[test]
fn quantized_tangent_and_morph_component_matrix_decodes_exact_values() {
    for encoding in [
        QuantizedEncoding::I8Normalized,
        QuantizedEncoding::I16Normalized,
        QuantizedEncoding::F32,
    ] {
        let (assets, scene_asset) = load_document(
            "quantized-tangent-matrix.gltf",
            quantized_component_fixture(QuantizedSemantic::Tangent, encoding),
        )
        .unwrap_or_else(|error| panic!("{encoding:?} tangent must load: {error}"));
        let geometry = assets
            .geometry(
                scene_asset.nodes()[0]
                    .mesh()
                    .expect("matrix tangent mesh")
                    .geometry(),
            )
            .expect("matrix tangent geometry");
        let tangent = geometry.tangents().expect("tangent stream")[0];
        assert_vec3_near(Vec3::new(tangent[0], tangent[1], tangent[2]), Vec3::X);
        assert_eq!(tangent[3], -1.0, "{encoding:?} handedness changed");
    }

    for encoding in [
        QuantizedEncoding::I8,
        QuantizedEncoding::I8Normalized,
        QuantizedEncoding::U8,
        QuantizedEncoding::U8Normalized,
        QuantizedEncoding::I16,
        QuantizedEncoding::I16Normalized,
        QuantizedEncoding::U16,
        QuantizedEncoding::U16Normalized,
        QuantizedEncoding::F32,
    ] {
        let (assets, scene_asset) = load_document(
            "quantized-morph-position-matrix.gltf",
            quantized_component_fixture(QuantizedSemantic::MorphPosition, encoding),
        )
        .unwrap_or_else(|error| panic!("{encoding:?} morph POSITION must load: {error}"));
        let geometry = assets
            .geometry(
                scene_asset.nodes()[0]
                    .mesh()
                    .expect("matrix morph mesh")
                    .geometry(),
            )
            .expect("matrix morph geometry");
        let actual = geometry.morph_targets()[0].position_deltas()[2].z;
        let expected = if encoding.is_normalized() || encoding == QuantizedEncoding::F32 {
            1.0
        } else {
            2.0
        };
        assert!(
            (actual - expected).abs() <= 1e-6,
            "{encoding:?} morph POSITION decoded {actual}, expected {expected}"
        );

        let mut scene = Scene::new();
        let import = scene
            .instantiate(&scene_asset)
            .expect("matrix morph instantiates");
        let node = import.node("MatrixMorph").expect("matrix morph node");
        let camera = scene.add_default_camera().expect("matrix camera inserts");
        scene
            .frame_import(camera, &import)
            .expect("matrix morph frames");
        scene
            .directional_light(DirectionalLight::key_light())
            .transform(Transform::default().rotate_x_deg(-45.0).rotate_y_deg(30.0))
            .add()
            .expect("matrix key light inserts");
        let mut renderer = Renderer::headless(24, 24).expect("matrix renderer builds");
        scene.set_morph_weights(node, [0.0]).unwrap();
        renderer.prepare_with_assets(&mut scene, &assets).unwrap();
        renderer.render(&scene, camera).unwrap();
        let base = renderer.frame_rgba8().to_vec();
        scene.set_morph_weights(node, [1.0]).unwrap();
        renderer.prepare_with_assets(&mut scene, &assets).unwrap();
        renderer.render(&scene, camera).unwrap();
        assert_ne!(
            renderer.frame_rgba8(),
            base.as_slice(),
            "{encoding:?} decoded morph POSITION must affect rendered output"
        );
    }

    for semantic in [
        QuantizedSemantic::MorphNormal,
        QuantizedSemantic::MorphTangent,
    ] {
        for encoding in [
            QuantizedEncoding::I8Normalized,
            QuantizedEncoding::I16Normalized,
            QuantizedEncoding::F32,
        ] {
            let (assets, scene_asset) = load_document(
                "quantized-morph-direction-matrix.gltf",
                quantized_component_fixture(semantic, encoding),
            )
            .unwrap_or_else(|error| panic!("{encoding:?} {semantic:?} must load: {error}"));
            let geometry = assets
                .geometry(
                    scene_asset.nodes()[0]
                        .mesh()
                        .expect("matrix directional morph mesh")
                        .geometry(),
                )
                .expect("matrix directional morph geometry");
            let target = &geometry.morph_targets()[0];
            let actual = match semantic {
                QuantizedSemantic::MorphNormal => {
                    target.normal_deltas().expect("normal deltas are present")[0]
                }
                QuantizedSemantic::MorphTangent => {
                    target.tangent_deltas().expect("tangent deltas are present")[0]
                }
                _ => unreachable!(),
            };
            assert_vec3_near(actual, Vec3::new(-1.0, 1.0, 0.0));
        }
    }
}

#[test]
fn quantized_accessors_honor_stride_and_sparse_overrides() {
    let (assets, scene_asset) = load_document(
        "quantized-strided-tangent.gltf",
        quantized_strided_tangent_fixture(),
    )
    .expect("strided quantized tangent loads");
    let geometry = assets
        .geometry(
            scene_asset.nodes()[0]
                .mesh()
                .expect("strided tangent mesh")
                .geometry(),
        )
        .expect("strided tangent geometry");
    for tangent in geometry.tangents().expect("strided tangent stream") {
        assert_vec3_near(Vec3::new(tangent[0], tangent[1], tangent[2]), Vec3::X);
        assert_eq!(tangent[3], -1.0);
    }

    let (assets, scene_asset) = load_document(
        "quantized-sparse-morph.gltf",
        quantized_sparse_morph_fixture(),
    )
    .expect("sparse quantized morph loads");
    let geometry = assets
        .geometry(
            scene_asset.nodes()[0]
                .mesh()
                .expect("sparse quantized morph mesh")
                .geometry(),
        )
        .expect("sparse quantized morph geometry");
    assert_eq!(
        geometry.morph_targets()[0].position_deltas(),
        &[Vec3::ZERO, Vec3::ZERO, Vec3::Z],
        "sparse override must retain target index and normalized value"
    );
}

#[test]
fn malformed_quantized_accessors_return_errors_without_panics() {
    for (label, fixture) in [
        (
            "truncated-quantized-tangent.gltf",
            malformed_quantized_tangent_fixture(false),
        ),
        (
            "overflow-quantized-tangent.gltf",
            malformed_quantized_tangent_fixture(true),
        ),
    ] {
        let result = std::panic::catch_unwind(|| load_document(label, fixture));
        assert!(result.is_ok(), "{label} must not panic");
        assert!(
            result.expect("panic already checked").is_err(),
            "{label} must fail closed"
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
fn morph_animation_width_must_match_target_geometry_before_playback() {
    for (name, target_count) in [("too-few", 3_usize), ("too-many", 1_usize)] {
        let mut document = cubic_weights_fixture();
        let targets = document["meshes"][0]["primitives"][0]["targets"]
            .as_array_mut()
            .expect("fixture targets are an array");
        if target_count == 3 {
            targets.push(targets[0].clone());
        } else {
            targets.truncate(1);
        }
        document["meshes"][0]["weights"] = Value::Array(vec![json!(0.0); target_count]);

        let error = expect_load_error(
            &format!("morph-animation-width-{name}.gltf"),
            document,
            "morph animation width mismatch must fail during asset loading",
        );
        assert!(matches!(
            error,
            AssetError::MorphWeightWidthMismatch {
                clip_index: 0,
                channel_index: 0,
                node_index: 0,
                primitive_index: 0,
                expected,
                actual: 2,
                ..
            } if expected == target_count
        ));
    }
}

#[test]
fn morph_animation_width_must_match_every_primitive() {
    let mut document = multi_primitive_fixture();
    let second_targets = document["meshes"][0]["primitives"][1]["targets"]
        .as_array_mut()
        .expect("second primitive targets are an array");
    second_targets.push(second_targets[0].clone());
    document["meshes"][0]["weights"] = json!([0.0, 0.0]);

    let error = expect_load_error(
        "multi-primitive-morph-animation-width.gltf",
        document,
        "every primitive must match the bound weight channel width",
    );
    assert!(matches!(
        error,
        AssetError::MorphWeightWidthMismatch {
            clip_index: 0,
            channel_index: 0,
            node_index: 0,
            primitive_index: 1,
            expected: 2,
            actual: 1,
            ..
        }
    ));
}

#[test]
fn hot_reload_rejects_changed_morph_animation_width_and_preserves_previous_asset() {
    let directory = std::env::temp_dir().join(format!(
        "scena-c22-morph-width-reload-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("reload fixture directory creates");
    let path = directory.join("scene.gltf");
    let valid = cubic_weights_fixture();
    std::fs::write(
        &path,
        serde_json::to_vec(&valid).expect("valid fixture serializes"),
    )
    .expect("valid reload source writes");

    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let original = pollster::block_on(assets.load_scene(path.to_string_lossy().as_ref()))
        .expect("valid morph animation loads before source changes");

    let mut invalid = valid;
    invalid["meshes"][0]["primitives"][0]["targets"]
        .as_array_mut()
        .expect("fixture targets are an array")
        .truncate(1);
    invalid["meshes"][0]["weights"] = json!([0.0]);
    std::fs::write(
        &path,
        serde_json::to_vec(&invalid).expect("invalid fixture serializes"),
    )
    .expect("changed reload source writes");

    let failure = pollster::block_on(assets.reload_scene_with_report(&original))
        .expect_err("changed target width must fail the reload transaction");
    assert!(failure.previous_asset_preserved());
    assert!(matches!(
        failure.error(),
        AssetError::MorphWeightWidthMismatch {
            clip_index: 0,
            channel_index: 0,
            node_index: 0,
            primitive_index: 0,
            expected: 1,
            actual: 2,
            ..
        }
    ));
    let cached = pollster::block_on(assets.load_scene(path.to_string_lossy().as_ref()))
        .expect("failed reload leaves the previous complete cache entry available");
    assert_eq!(cached.provenance(), original.provenance());
    std::fs::remove_dir_all(&directory).expect("reload fixture directory removes");
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

fn missing_normal_hard_edge_fixture(indexed: bool, degenerate: bool) -> Value {
    let positions = if degenerate {
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
        ]
    } else if indexed {
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ]
    } else {
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
        ]
    };
    let mut bytes = Vec::new();
    for position in &positions {
        push_f32s(&mut bytes, position);
    }
    let positions_len = bytes.len();
    let indices = indexed.then(|| {
        let offset = bytes.len();
        push_u16s(&mut bytes, &[0, 1, 2, 0, 3, 1]);
        (offset, 12)
    });
    let min = positions.iter().fold([f32::INFINITY; 3], |mut min, value| {
        for axis in 0..3 {
            min[axis] = min[axis].min(value[axis]);
        }
        min
    });
    let max = positions
        .iter()
        .fold([f32::NEG_INFINITY; 3], |mut max, value| {
            for axis in 0..3 {
                max[axis] = max[axis].max(value[axis]);
            }
            max
        });
    let mut views = vec![json!({"buffer":0,"byteOffset":0,"byteLength":positions_len})];
    let mut accessors = vec![json!({
        "bufferView":0,"componentType":5126,"count":positions.len(),"type":"VEC3",
        "min":min,"max":max
    })];
    let mut primitive = json!({"attributes":{"POSITION":0}});
    if let Some((offset, length)) = indices {
        views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":length}));
        accessors.push(json!({"bufferView":1,"componentType":5123,"count":6,"type":"SCALAR"}));
        primitive["indices"] = json!(1);
    }
    json!({
        "asset":{"version":"2.0"},
        "nodes":[{"name":"MissingNormal","mesh":0}],
        "meshes":[{"primitives":[primitive]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":views,
        "accessors":accessors
    })
}

fn texture_coordinate_one_fixture(slot: &str) -> Value {
    let mut bytes = triangle_positions_indices();
    let tex_coords1 = bytes.len();
    push_f32s(&mut bytes, &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    let mut material = json!({"pbrMetallicRoughness":{}});
    match slot {
        "baseColorTexture" | "metallicRoughnessTexture" => {
            material["pbrMetallicRoughness"][slot] = json!({"index":0,"texCoord":1});
        }
        "normalTexture" | "occlusionTexture" | "emissiveTexture" => {
            material[slot] = json!({"index":0,"texCoord":1});
        }
        "clearcoatTexture" => {
            material["extensions"] = json!({
                "KHR_materials_clearcoat":{"clearcoatTexture":{"index":0,"texCoord":1}}
            });
        }
        _ => unreachable!("fixture slot is enumerated by the test"),
    }
    let mut document = json!({
        "asset":{"version":"2.0"},
        "nodes":[{"name":"UvSet","mesh":0}],
        "meshes":[{"primitives":[{
            "attributes":{"POSITION":0,"TEXCOORD_1":2},"indices":1,"material":0
        }]}],
        "materials":[material],
        "images":[{"uri":normal_map_uri()}],
        "textures":[{"source":0}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":6},
            {"buffer":0,"byteOffset":tex_coords1,"byteLength":24}
        ],
        "accessors":[
            position_accessor(),index_accessor(),
            {"bufferView":2,"componentType":5126,"count":3,"type":"VEC2"}
        ]
    });
    if slot == "clearcoatTexture" {
        document["extensionsUsed"] = json!(["KHR_materials_clearcoat"]);
    }
    document
}

fn eight_skin_influences_fixture() -> Value {
    skin_influences_fixture([4, 5, 6, 7], [0.4, 0.03, 0.06, 0.01])
}

fn skin_influences_fixture(secondary_joints: [u16; 4], secondary_weights: [f32; 4]) -> Value {
    let mut bytes = triangle_positions_indices();
    let joints0 = bytes.len();
    for _ in 0..3 {
        push_u16s(&mut bytes, &[0, 1, 2, 3]);
    }
    let weights0 = bytes.len();
    for _ in 0..3 {
        push_f32s(&mut bytes, &[0.05, 0.2, 0.1, 0.15]);
    }
    let joints1 = bytes.len();
    for _ in 0..3 {
        push_u16s(&mut bytes, &secondary_joints);
    }
    let weights1 = bytes.len();
    for _ in 0..3 {
        push_f32s(&mut bytes, &secondary_weights);
    }
    let mut nodes = vec![json!({"name":"Skinned","mesh":0,"skin":0})];
    nodes.extend((0..8).map(|index| json!({"name":format!("Joint{index}")})));
    json!({
        "asset":{"version":"2.0"},
        "nodes":nodes,
        "skins":[{"joints":[1,2,3,4,5,6,7,8]}],
        "meshes":[{"primitives":[{
            "attributes":{
                "POSITION":0,"JOINTS_0":2,"WEIGHTS_0":3,"JOINTS_1":4,"WEIGHTS_1":5
            },
            "indices":1
        }]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":6},
            {"buffer":0,"byteOffset":joints0,"byteLength":24},
            {"buffer":0,"byteOffset":weights0,"byteLength":48},
            {"buffer":0,"byteOffset":joints1,"byteLength":24},
            {"buffer":0,"byteOffset":weights1,"byteLength":48}
        ],
        "accessors":[
            position_accessor(),index_accessor(),
            {"bufferView":2,"componentType":5123,"count":3,"type":"VEC4"},
            {"bufferView":3,"componentType":5126,"count":3,"type":"VEC4"},
            {"bufferView":4,"componentType":5123,"count":3,"type":"VEC4"},
            {"bufferView":5,"componentType":5126,"count":3,"type":"VEC4"}
        ]
    })
}

fn shared_mesh_node_morph_override_fixture() -> Value {
    let mut document = multi_primitive_fixture();
    document["nodes"] = json!([
        {"name":"OverrideA","mesh":0,"weights":[0.75]},
        {"name":"OverrideB","mesh":0,"weights":[0.25]},
        {"name":"MeshDefault","mesh":0}
    ]);
    document["meshes"][0]["weights"] = json!([0.1]);
    document["animations"][0]["name"] = json!("OverrideAnimation");
    document
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

fn quantized_tangent_morph_fixture() -> Value {
    let mut bytes = triangle_positions_indices();
    let tangents = bytes.len();
    for _ in 0..3 {
        bytes.extend_from_slice(&[127_u8, 0, 0, 129]);
    }
    let morph_positions = bytes.len();
    for value in [0_i16, 0, 0, 0, 0, 0, 0, 0, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let morph_normals = bytes.len();
    for _ in 0..3 {
        bytes.extend_from_slice(&[127_u8, 0, 129]);
    }
    let morph_tangents = bytes.len();
    for _ in 0..3 {
        bytes.extend_from_slice(&[129_u8, 127, 0]);
    }
    let views = [
        (0, 36),
        (36, 6),
        (tangents, 12),
        (morph_positions, 18),
        (morph_normals, 9),
        (morph_tangents, 9),
    ];
    json!({
        "asset":{"version":"2.0"},
        "extensionsUsed":["KHR_mesh_quantization"],
        "extensionsRequired":["KHR_mesh_quantization"],
        "nodes":[{"name":"QuantizedMorph","mesh":0}],
        "meshes":[{"weights":[0.0],"primitives":[{
            "attributes":{"POSITION":0,"TANGENT":2},
            "indices":1,
            "targets":[{"POSITION":3,"NORMAL":4,"TANGENT":5}]
        }]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":views.iter().map(|(offset,length)|json!({
            "buffer":0,"byteOffset":offset,"byteLength":length
        })).collect::<Vec<_>>(),
        "accessors":[
            position_accessor(),
            index_accessor(),
            {"bufferView":2,"componentType":5120,"normalized":true,"count":3,"type":"VEC4"},
            {"bufferView":3,"componentType":5122,"count":3,"type":"VEC3"},
            {"bufferView":4,"componentType":5120,"normalized":true,"count":3,"type":"VEC3"},
            {"bufferView":5,"componentType":5120,"normalized":true,"count":3,"type":"VEC3"}
        ]
    })
}

fn non_finite_mesh_stream_fixture(morph: bool) -> Value {
    let mut bytes = triangle_positions_indices();
    let stream_offset = bytes.len();
    if morph {
        for values in [[0.0, 0.0, 0.0], [f32::INFINITY, 0.0, 0.0], [0.0, 0.0, 0.0]] {
            push_f32s(&mut bytes, &values);
        }
    } else {
        for values in [
            [1.0, 0.0, 0.0, 1.0],
            [f32::NAN, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
        ] {
            push_f32s(&mut bytes, &values);
        }
    }
    let stream_length = bytes.len() - stream_offset;
    let stream_type = if morph { "VEC3" } else { "VEC4" };
    let primitive = if morph {
        json!({"attributes":{"POSITION":0},"indices":1,"targets":[{"POSITION":2}]})
    } else {
        json!({"attributes":{"POSITION":0,"TANGENT":2},"indices":1})
    };
    json!({
        "asset":{"version":"2.0"},
        "nodes":[{"name":"MatrixMorph","mesh":0}],
        "meshes":[{"primitives":[primitive]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":6},
            {"buffer":0,"byteOffset":stream_offset,"byteLength":stream_length}
        ],
        "accessors":[
            position_accessor(),
            index_accessor(),
            {"bufferView":2,"componentType":5126,"count":3,"type":stream_type}
        ]
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuantizedEncoding {
    I8,
    I8Normalized,
    U8,
    U8Normalized,
    I16,
    I16Normalized,
    U16,
    U16Normalized,
    F32,
}

impl QuantizedEncoding {
    const fn component_type(self) -> u32 {
        match self {
            Self::I8 | Self::I8Normalized => 5120,
            Self::U8 | Self::U8Normalized => 5121,
            Self::I16 | Self::I16Normalized => 5122,
            Self::U16 | Self::U16Normalized => 5123,
            Self::F32 => 5126,
        }
    }

    const fn is_normalized(self) -> bool {
        matches!(
            self,
            Self::I8Normalized | Self::U8Normalized | Self::I16Normalized | Self::U16Normalized
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum QuantizedSemantic {
    Tangent,
    MorphPosition,
    MorphNormal,
    MorphTangent,
}

fn quantized_component_fixture(semantic: QuantizedSemantic, encoding: QuantizedEncoding) -> Value {
    let mut bytes = triangle_positions_indices();
    let stream_offset = bytes.len();
    let vectors = match semantic {
        QuantizedSemantic::Tangent => vec![[1.0, 0.0, 0.0, -1.0]; 3],
        QuantizedSemantic::MorphPosition => vec![
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
        QuantizedSemantic::MorphNormal | QuantizedSemantic::MorphTangent => {
            vec![[-1.0, 1.0, 0.0, 0.0]; 3]
        }
    };
    for vector in &vectors {
        push_quantized_vector(
            &mut bytes,
            encoding,
            vector,
            matches!(semantic, QuantizedSemantic::Tangent),
        );
    }
    let stream_length = bytes.len() - stream_offset;
    let stream_type = if matches!(semantic, QuantizedSemantic::Tangent) {
        "VEC4"
    } else {
        "VEC3"
    };
    let (attributes, targets) = match semantic {
        QuantizedSemantic::Tangent => (json!({"POSITION":0,"TANGENT":2}), None),
        QuantizedSemantic::MorphPosition => (json!({"POSITION":0}), Some(json!([{"POSITION":2}]))),
        QuantizedSemantic::MorphNormal => (json!({"POSITION":0}), Some(json!([{"NORMAL":2}]))),
        QuantizedSemantic::MorphTangent => (json!({"POSITION":0}), Some(json!([{"TANGENT":2}]))),
    };
    let mut primitive = json!({"attributes":attributes,"indices":1});
    if let Some(targets) = targets {
        primitive["targets"] = targets;
    }
    json!({
        "asset":{"version":"2.0"},
        "extensionsUsed":["KHR_mesh_quantization"],
        "nodes":[{"name":"MatrixMorph","mesh":0}],
        "meshes":[{"weights":[0.0],"primitives":[primitive]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":6},
            {"buffer":0,"byteOffset":stream_offset,"byteLength":stream_length}
        ],
        "accessors":[
            position_accessor(),
            index_accessor(),
            {"bufferView":2,"componentType":encoding.component_type(),"normalized":encoding.is_normalized(),"count":3,"type":stream_type}
        ]
    })
}

fn push_quantized_vector(
    bytes: &mut Vec<u8>,
    encoding: QuantizedEncoding,
    vector: &[f32; 4],
    vec4: bool,
) {
    let components = if vec4 { 4 } else { 3 };
    for value in &vector[..components] {
        match encoding {
            QuantizedEncoding::I8 | QuantizedEncoding::I8Normalized => {
                let value = if encoding.is_normalized() {
                    if *value < 0.0 {
                        i8::MIN
                    } else if *value > 0.0 {
                        i8::MAX
                    } else {
                        0
                    }
                } else if *value == 1.0 {
                    2
                } else {
                    *value as i8
                };
                bytes.push(value as u8);
            }
            QuantizedEncoding::U8 | QuantizedEncoding::U8Normalized => {
                let value = if encoding.is_normalized() {
                    if *value > 0.0 { u8::MAX } else { 0 }
                } else if *value == 1.0 {
                    2
                } else {
                    *value as u8
                };
                bytes.push(value);
            }
            QuantizedEncoding::I16 | QuantizedEncoding::I16Normalized => {
                let value = if encoding.is_normalized() {
                    if *value < 0.0 {
                        i16::MIN
                    } else if *value > 0.0 {
                        i16::MAX
                    } else {
                        0
                    }
                } else if *value == 1.0 {
                    2
                } else {
                    *value as i16
                };
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            QuantizedEncoding::U16 | QuantizedEncoding::U16Normalized => {
                let value = if encoding.is_normalized() {
                    if *value > 0.0 { u16::MAX } else { 0 }
                } else if *value == 1.0 {
                    2
                } else {
                    *value as u16
                };
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            QuantizedEncoding::F32 => bytes.extend_from_slice(&value.to_le_bytes()),
        }
    }
}

fn quantized_strided_tangent_fixture() -> Value {
    let mut bytes = triangle_positions_indices();
    let stream_offset = bytes.len();
    for _ in 0..3 {
        bytes.extend_from_slice(&[127, 0, 0, 128, 0xaa, 0xbb, 0xcc, 0xdd]);
    }
    json!({
        "asset":{"version":"2.0"},"extensionsUsed":["KHR_mesh_quantization"],
        "nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"TANGENT":2},"indices":1}]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":6},
            {"buffer":0,"byteOffset":stream_offset,"byteLength":24,"byteStride":8}
        ],
        "accessors":[position_accessor(),index_accessor(),{"bufferView":2,"componentType":5120,"normalized":true,"count":3,"type":"VEC4"}]
    })
}

fn quantized_sparse_morph_fixture() -> Value {
    let mut bytes = triangle_positions_indices();
    let base = bytes.len();
    bytes.extend_from_slice(&[0_u8; 9]);
    let sparse_index = bytes.len();
    bytes.push(2);
    let sparse_value = bytes.len();
    bytes.extend_from_slice(&[0, 0, 255]);
    json!({
        "asset":{"version":"2.0"},"extensionsUsed":["KHR_mesh_quantization"],
        "nodes":[{"mesh":0}],"meshes":[{"weights":[0.0],"primitives":[{"attributes":{"POSITION":0},"indices":1,"targets":[{"POSITION":2}]}]}],
        "buffers":[{"byteLength":bytes.len(),"uri":data_uri(&bytes)}],
        "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":6},
            {"buffer":0,"byteOffset":base,"byteLength":9},
            {"buffer":0,"byteOffset":sparse_index,"byteLength":1},
            {"buffer":0,"byteOffset":sparse_value,"byteLength":3}
        ],
        "accessors":[
            position_accessor(),index_accessor(),
            {"bufferView":2,"componentType":5121,"normalized":true,"count":3,"type":"VEC3","sparse":{"count":1,"indices":{"bufferView":3,"componentType":5121},"values":{"bufferView":4}}}
        ]
    })
}

fn malformed_quantized_tangent_fixture(overflow: bool) -> Value {
    let mut fixture = quantized_strided_tangent_fixture();
    if overflow {
        fixture["accessors"][2]["byteOffset"] = json!(u64::MAX);
    } else {
        fixture["bufferViews"][2]["byteLength"] = json!(16);
    }
    fixture
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

fn load_document_report(
    name: &str,
    document: Value,
) -> Result<(Assets<MemoryFetcher>, AssetLoadReport<SceneAsset>), AssetError> {
    let path = AssetPath::from(format!("memory://c04/{name}"));
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        path.clone(),
        serde_json::to_vec(&document).unwrap(),
    ));
    let report = pollster::block_on(assets.load_scene_with_report(path.as_str()))?;
    Ok((assets, report))
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
