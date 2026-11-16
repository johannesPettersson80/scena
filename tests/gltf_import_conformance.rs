#![cfg(not(target_arch = "wasm32"))]

//! Conformance contracts for the glTF importer: constructs the glTF 2.0
//! specification allows must import as the specification describes them, not
//! be dropped, altered, or rejected.

use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::Arc;

use base64::Engine as _;
use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, Color, Light, LightKey, NodeKey, NodeKind, Scene,
    SceneAsset, Transform, Vec3,
};
use serde_json::{Value, json};

const UNSIGNED_SHORT: u32 = 5123;
const FLOAT: u32 = 5126;

#[test]
fn a_node_with_a_mesh_and_a_light_instantiates_both() {
    // KHR_lights_punctual attaches a light to a node; nothing stops that node
    // from also carrying a mesh (a lamp model with its bulb, for example).
    let mut buffers = Buffers::default();
    let position = buffers.positions(&TRIANGLE);
    let normal = buffers.vec3(&[[0.0, 0.0, 1.0]; 3]);
    let instance_offsets = buffers.vec3(&[[0.0, 0.0, 0.0], [4.0, 0.0, 0.0]]);
    let primitive = json!({"attributes": {"POSITION": position, "NORMAL": normal}});
    let document = buffers.finish(json!({
        "asset": {"version": "2.0"},
        "extensionsUsed": ["KHR_lights_punctual", "EXT_mesh_gpu_instancing"],
        "extensions": {"KHR_lights_punctual": {"lights": [
            {"type": "point", "color": [1.0, 0.5, 0.25], "intensity": 60.0, "range": 8.0},
            {"type": "spot", "intensity": 100.0,
             "spot": {"innerConeAngle": 0.2, "outerConeAngle": 0.6}},
            {"type": "directional", "intensity": 3.0},
        ]}},
        "scene": 0,
        "scenes": [{"nodes": [0, 1, 2]}],
        "nodes": [
            {"name": "Lamp", "mesh": 0, "translation": [1.0, 2.0, 3.0],
             "extensions": {"KHR_lights_punctual": {"light": 0}}},
            {"name": "Chandelier", "mesh": 1, "translation": [-1.0, 0.0, 0.0],
             "extensions": {"KHR_lights_punctual": {"light": 1}}},
            {"name": "Sconces", "mesh": 0, "translation": [0.0, 5.0, 0.0],
             "extensions": {
                 "KHR_lights_punctual": {"light": 2},
                 "EXT_mesh_gpu_instancing": {"attributes": {"TRANSLATION": instance_offsets}},
             }},
        ],
        "meshes": [
            {"primitives": [primitive.clone()]},
            {"primitives": [primitive.clone(), primitive]},
        ],
    }));
    let assets = assets_with([("lit-meshes.gltf", document)]);
    let scene_asset = load(&assets, "lit-meshes.gltf").expect("the fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("the fixture instantiates");

    // A single-primitive node stays a mesh node and gains the light as a child.
    let lamp = import.node("Lamp").expect("the lamp node is addressable");
    assert!(
        matches!(
            scene.node(lamp).expect("lamp exists").kind(),
            NodeKind::Mesh(_)
        ),
        "the node keeps its mesh",
    );
    let lamp_light = only_light_child(&scene, lamp);
    let lamp_light_node = lamp_light.node;
    let Light::Point(point) = lamp_light.light else {
        panic!(
            "the lamp declares a point light, got {:?}",
            lamp_light.light
        );
    };
    assert_eq!(point.color(), Color::from_linear_rgb(1.0, 0.5, 0.25));
    assert_eq!(point.intensity_candela(), 60.0);
    assert_eq!(point.range(), Some(8.0));
    assert_eq!(
        scene
            .node(lamp_light_node)
            .expect("light exists")
            .transform(),
        Transform::IDENTITY,
        "the light shines from the node's own frame",
    );
    assert_eq!(
        scene
            .world_transform(lamp_light_node)
            .expect("light has a world transform")
            .translation,
        Vec3::new(1.0, 2.0, 3.0),
    );

    // A multi-primitive node keeps one child per primitive and gains the light.
    let chandelier = import
        .node("Chandelier")
        .expect("the chandelier node is addressable");
    let chandelier_light = only_light_child(&scene, chandelier);
    assert!(
        matches!(chandelier_light.light, Light::Spot(_)),
        "the chandelier declares a spot light, got {:?}",
        chandelier_light.light,
    );
    let primitive_children = scene
        .node(chandelier)
        .expect("chandelier exists")
        .children()
        .iter()
        .filter(|child| {
            matches!(
                scene.node(**child).expect("child exists").kind(),
                NodeKind::Mesh(_)
            )
        })
        .count();
    assert_eq!(primitive_children, 2, "both primitives are still imported");

    // An instanced mesh keeps its instances and gains one light at the node.
    let sconces = import
        .node("Sconces")
        .expect("the instanced node is addressable");
    assert!(
        matches!(
            scene.node(sconces).expect("sconces exist").kind(),
            NodeKind::InstanceSet(_)
        ),
        "the node keeps its instanced mesh",
    );
    let sconce_light = only_light_child(&scene, sconces);
    assert!(
        matches!(sconce_light.light, Light::Directional(_)),
        "the instanced node declares a directional light, got {:?}",
        sconce_light.light,
    );
    assert_eq!(
        scene
            .world_transform(sconce_light.node)
            .expect("light has a world transform")
            .translation,
        Vec3::new(0.0, 5.0, 0.0),
        "the light sits at the node, not at an instance",
    );

    // The lights belong to the import and leave the scene with it.
    scene.remove_import(&import).expect("the import is removed");
    for imported in [lamp_light, chandelier_light, sconce_light] {
        assert!(scene.node(imported.node).is_none());
        assert!(scene.light(imported.key).is_none());
    }
}

#[derive(Debug, Clone, Copy)]
struct ImportedLight {
    node: NodeKey,
    key: LightKey,
    light: Light,
}

/// The single light node directly below `node`, with the light it holds.
fn only_light_child(scene: &Scene, node: NodeKey) -> ImportedLight {
    let lights = scene
        .node(node)
        .expect("node exists")
        .children()
        .iter()
        .filter_map(|child| match scene.node(*child)?.kind() {
            NodeKind::Light(key) => Some(ImportedLight {
                node: *child,
                key: *key,
                light: *scene.light(*key)?,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lights.len(),
        1,
        "the node must carry exactly one light child, found {lights:?}",
    );
    lights[0]
}

const TRIANGLE: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

/// Binary payload and accessors of an embedded glTF document.
#[derive(Default)]
struct Buffers {
    bytes: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Buffers {
    fn accessor(&mut self, data: &[u8], component_type: u32, kind: &str, count: usize) -> usize {
        while !self.bytes.len().is_multiple_of(4) {
            self.bytes.push(0);
        }
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": self.bytes.len(),
            "byteLength": data.len(),
        }));
        self.bytes.extend_from_slice(data);
        self.accessors.push(json!({
            "bufferView": self.views.len() - 1,
            "componentType": component_type,
            "count": count,
            "type": kind,
        }));
        self.accessors.len() - 1
    }

    fn vec3(&mut self, values: &[[f32; 3]]) -> usize {
        let data = values
            .iter()
            .flatten()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        self.accessor(&data, FLOAT, "VEC3", values.len())
    }

    /// A POSITION accessor, which must declare its bounds.
    fn positions(&mut self, values: &[[f32; 3]]) -> usize {
        let accessor = self.vec3(values);
        let bound = |pick: fn(f32, f32) -> f32, start: f32| {
            (0..3)
                .map(|axis| {
                    values
                        .iter()
                        .fold(start, |acc, value| pick(acc, value[axis]))
                })
                .collect::<Vec<_>>()
        };
        self.accessors[accessor]["min"] = json!(bound(f32::min, f32::INFINITY));
        self.accessors[accessor]["max"] = json!(bound(f32::max, f32::NEG_INFINITY));
        accessor
    }

    #[allow(dead_code)]
    fn indices(&mut self, values: &[u16]) -> usize {
        let data = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        self.accessor(&data, UNSIGNED_SHORT, "SCALAR", values.len())
    }

    /// Adds the accessors, views and the embedded buffer to `document`.
    fn finish(self, mut document: Value) -> Value {
        document["accessors"] = json!(self.accessors);
        document["bufferViews"] = json!(self.views);
        document["buffers"] = json!([{
            "byteLength": self.bytes.len(),
            "uri": format!(
                "data:application/octet-stream;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&self.bytes)
            ),
        }]);
        document
    }
}

fn memory_path(name: &str) -> AssetPath {
    AssetPath::from(format!("memory://gltf-conformance/{name}"))
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
