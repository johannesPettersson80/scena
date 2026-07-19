#![cfg(not(target_arch = "wasm32"))]

use base64::Engine as _;
use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, ConnectOptions, ConnectorFrame, ImportOptions,
    Scene, SourceUnits, Transform, Vec3,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::Arc;

#[test]
fn nested_non_meter_imports_apply_units_once_to_translations_bounds_and_scale() {
    let (assets, scene_asset) = load_document("nested-units.gltf", nested_mesh_fixture());

    for units in [
        SourceUnits::Millimeters,
        SourceUnits::Centimeters,
        SourceUnits::Inches,
        SourceUnits::Feet,
    ] {
        let unit_scale = units.meters_per_unit();
        let mut scene = Scene::new();
        let import = scene
            .instantiate_with(
                &scene_asset,
                ImportOptions::gltf_default().with_source_units(units),
            )
            .unwrap_or_else(|error| panic!("{units:?} import must instantiate: {error}"));
        let unit_root = import.roots()[0];
        let root = import.node("Root").expect("source root resolves");
        let middle = import.node("Middle").expect("middle node resolves");
        let mesh = import.node("Mesh").expect("mesh node resolves");

        assert_ne!(
            unit_root, root,
            "non-meter conversion belongs to one synthetic import root"
        );
        assert_transform_near(
            scene.node(unit_root).expect("unit root exists").transform(),
            Transform::IDENTITY.scale_by(unit_scale),
        );
        assert_transform_near(
            scene.node(root).expect("source root exists").transform(),
            Transform {
                translation: Vec3::new(100.0, 0.0, 0.0),
                scale: Vec3::splat(2.0),
                ..Transform::IDENTITY
            },
        );
        assert_transform_near(
            scene.node(middle).expect("middle exists").transform(),
            Transform {
                translation: Vec3::new(50.0, 0.0, 0.0),
                scale: Vec3::splat(3.0),
                ..Transform::IDENTITY
            },
        );
        assert_transform_near(
            scene.node(mesh).expect("mesh exists").transform(),
            Transform {
                translation: Vec3::new(25.0, 0.0, 0.0),
                scale: Vec3::splat(4.0),
                ..Transform::IDENTITY
            },
        );

        let world = scene.world_transform(mesh).expect("mesh world transform");
        assert_vec3_near(world.translation, Vec3::new(350.0 * unit_scale, 0.0, 0.0));
        assert_vec3_near(world.scale, Vec3::splat(24.0 * unit_scale));

        let bounds = scene
            .node_world_bounds(unit_root, &assets)
            .expect("unit root bounds resolve")
            .expect("mesh produces bounds");
        assert_vec3_near(bounds.min, Vec3::new(350.0 * unit_scale, 0.0, 0.0));
        assert_vec3_near(
            bounds.max,
            Vec3::new(590.0 * unit_scale, 240.0 * unit_scale, 0.0),
        );
    }
}

#[test]
fn non_meter_import_scale_animation_remains_dimensionless() {
    let (_assets, scene_asset) = load_document("animated-scale.gltf", animated_scale_fixture());
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default().with_source_units(SourceUnits::Millimeters),
        )
        .expect("millimeter animation fixture instantiates");
    let target = import
        .node("AnimatedScale")
        .expect("animated node resolves");
    let mixer = scene
        .create_animation_mixer(&import, "Scale")
        .expect("scale mixer creates");

    scene.seek_animation(mixer, 1.0).expect("scale clip seeks");

    assert_vec3_near(
        scene
            .node(target)
            .expect("animated node exists")
            .transform()
            .scale,
        Vec3::splat(2.0),
    );
    assert_vec3_near(
        scene
            .world_transform(target)
            .expect("animated node world transform")
            .scale,
        Vec3::splat(0.002),
    );
}

#[test]
fn nested_inherited_and_explicit_marker_units_align_in_world_space() {
    let (_assets, scene_asset) = load_document("nested-markers.gltf", nested_marker_fixture());
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default().with_source_units(SourceUnits::Millimeters),
        )
        .expect("nested marker fixture instantiates");
    let inherited = import
        .anchor("inherited")
        .expect("inherited anchor resolves");
    let explicit = import
        .anchor("explicit-meter")
        .expect("explicit anchor resolves");
    let connector = import
        .connector("inherited-connector")
        .expect("connector resolves");

    assert_eq!(inherited.source_units(), SourceUnits::Millimeters);
    assert_eq!(explicit.source_units(), SourceUnits::Meters);
    assert_eq!(connector.source_units(), SourceUnits::Millimeters);
    assert_eq!(inherited.placement_node(), import.roots()[0]);
    assert_eq!(explicit.placement_node(), import.roots()[0]);
    assert_eq!(connector.placement_node(), import.roots()[0]);

    let inherited_source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("inherited source inserts");
    scene
        .connect(
            ConnectorFrame::new(inherited_source, Transform::IDENTITY),
            ConnectorFrame::from_import_anchor(inherited),
            ConnectOptions::default(),
        )
        .expect("inherited-unit anchor is compatible with a meter host");
    assert_vec3_near(
        scene.world_transform(inherited_source).unwrap().translation,
        Vec3::new(0.175, 0.0, 0.0),
    );

    let explicit_source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("explicit source inserts");
    scene
        .connect(
            ConnectorFrame::new(explicit_source, Transform::IDENTITY),
            ConnectorFrame::from_import_anchor(explicit),
            ConnectOptions::default(),
        )
        .expect("explicit meter anchor is compatible with a meter host");
    assert_vec3_near(
        scene.world_transform(explicit_source).unwrap().translation,
        Vec3::new(1.15, 0.0, 0.0),
    );

    let connector_source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("connector source inserts");
    scene
        .connect(
            ConnectorFrame::new(connector_source, Transform::IDENTITY),
            ConnectorFrame::from_import_connector(connector),
            ConnectOptions::default(),
        )
        .expect("inherited-unit connector is compatible with a meter host");
    assert_vec3_near(
        scene.world_transform(connector_source).unwrap().translation,
        Vec3::new(0.18, 0.0, 0.0),
    );
}

#[test]
fn marker_locals_stay_in_import_units_until_the_single_unit_root() {
    let (_assets, scene_asset) = load_document("marker-regression.gltf", nested_marker_fixture());
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default().with_source_units(SourceUnits::Millimeters),
        )
        .expect("marker regression fixture instantiates");
    let inherited = import.anchor("inherited").unwrap();
    let explicit = import.anchor("explicit-meter").unwrap();
    let connector = import.connector("inherited-connector").unwrap();

    assert_vec3_near(inherited.transform().translation, Vec3::new(25.0, 0.0, 0.0));
    assert_vec3_near(
        explicit.transform().translation,
        Vec3::new(1000.0, 0.0, 0.0),
    );
    assert_vec3_near(connector.transform().translation, Vec3::new(30.0, 0.0, 0.0));

    let child_world = scene
        .world_transform(explicit.node())
        .expect("explicit anchor host has world transform");
    let marker_world = Transform::compose(child_world, explicit.transform());
    assert_vec3_near(marker_world.translation, Vec3::new(1.15, 0.0, 0.0));
    assert!(
        (marker_world.translation.x - 0.151).abs() > 0.9,
        "pre-converting the 1 m marker to a 1.0 local before the 0.001 unit root would double-convert it"
    );
}

fn nested_mesh_fixture() -> Value {
    let mut bytes = Vec::new();
    push_f32s(&mut bytes, &[0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0]);
    json!({
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Root", "translation": [100.0, 0.0, 0.0], "scale": [2.0, 2.0, 2.0], "children": [1] },
            { "name": "Middle", "translation": [50.0, 0.0, 0.0], "scale": [3.0, 3.0, 3.0], "children": [2] },
            { "name": "Mesh", "translation": [25.0, 0.0, 0.0], "scale": [4.0, 4.0, 4.0], "mesh": 0 }
        ],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
        "buffers": [{ "byteLength": bytes.len(), "uri": data_uri(&bytes) }],
        "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
        "accessors": [{
            "bufferView": 0,
            "componentType": 5126,
            "count": 3,
            "type": "VEC3",
            "min": [0.0, 0.0, 0.0],
            "max": [10.0, 10.0, 0.0]
        }]
    })
}

fn animated_scale_fixture() -> Value {
    let mut bytes = Vec::new();
    push_f32s(&mut bytes, &[0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0]);
    push_f32s(&mut bytes, &[0.0, 1.0]);
    push_f32s(&mut bytes, &[1.0, 1.0, 1.0, 2.0, 2.0, 2.0]);
    json!({
        "asset": { "version": "2.0" },
        "nodes": [{ "name": "AnimatedScale", "mesh": 0 }],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
        "animations": [{
            "name": "Scale",
            "samplers": [{ "input": 1, "output": 2, "interpolation": "LINEAR" }],
            "channels": [{ "sampler": 0, "target": { "node": 0, "path": "scale" } }]
        }],
        "buffers": [{ "byteLength": bytes.len(), "uri": data_uri(&bytes) }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 44, "byteLength": 24 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [10.0, 10.0, 0.0] },
            { "bufferView": 1, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC3" }
        ]
    })
}

fn nested_marker_fixture() -> Value {
    json!({
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Root", "translation": [100.0, 0.0, 0.0], "children": [1] },
            {
                "name": "MarkerHost",
                "translation": [50.0, 0.0, 0.0],
                "extras": {
                    "scena": {
                        "anchors": [
                            { "name": "inherited", "translation": [25.0, 0.0, 0.0] },
                            { "name": "explicit-meter", "translation": [1.0, 0.0, 0.0], "units": "meters" }
                        ],
                        "connectors": [
                            { "name": "inherited-connector", "kind": "mount", "translation": [30.0, 0.0, 0.0] }
                        ]
                    }
                }
            }
        ]
    })
}

fn load_document(name: &str, document: Value) -> (Assets<MemoryFetcher>, scena::SceneAsset) {
    let path = AssetPath::from(format!("memory://c05/{name}"));
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        path.clone(),
        serde_json::to_vec(&document).expect("fixture serializes"),
    ));
    let scene_asset = pollster::block_on(assets.load_scene(path.as_str()))
        .unwrap_or_else(|error| panic!("{name} loads: {error}"));
    (assets, scene_asset)
}

fn data_uri(bytes: &[u8]) -> String {
    format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes),
    )
}

fn push_f32s(bytes: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn assert_transform_near(actual: Transform, expected: Transform) {
    assert_vec3_near(actual.translation, expected.translation);
    assert_vec3_near(actual.scale, expected.scale);
    let dot = actual.rotation.dot(expected.rotation).abs();
    assert!((dot - 1.0).abs() <= 1.0e-5, "{actual:?} != {expected:?}");
}

fn assert_vec3_near(actual: Vec3, expected: Vec3) {
    assert!(
        actual.abs_diff_eq(expected, 1.0e-4),
        "expected {actual:?} near {expected:?}"
    );
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
