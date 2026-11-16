#![cfg(not(target_arch = "wasm32"))]

//! Conformance contracts for the glTF importer: constructs the glTF 2.0
//! specification allows must import as the specification describes them, not
//! be dropped, altered, or rejected.

use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::Arc;

use base64::Engine as _;
use scena::{
    AssetError, AssetFetcher, AssetLoadOptions, AssetPath, Assets, Color, GeometryDesc,
    GeometryTopology, Light, LightKey, NodeKey, NodeKind, Scene, SceneAsset, Transform, Vec3,
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

#[test]
fn triangle_strips_and_fans_import_as_triangle_lists() {
    // glTF: strip triangle i is (i, i+1, i+2) when i is even and (i, i+2, i+1)
    // when i is odd, which keeps every triangle wound the same way; fan
    // triangle i is (i+1, i+2, 0).
    let strip = primitive_geometry(MeshMode::TriangleStrip, &RIBBON, None, true);
    assert_eq!(strip.topology(), GeometryTopology::Triangles);
    assert_eq!(strip.indices(), [0, 1, 2, 1, 3, 2, 2, 3, 4]);
    assert_eq!(strip.vertices().len(), RIBBON.len());

    let indexed_strip = primitive_geometry(
        MeshMode::TriangleStrip,
        &RIBBON,
        Some(&[4, 3, 2, 1, 0]),
        true,
    );
    assert_eq!(indexed_strip.indices(), [4, 3, 2, 3, 1, 2, 2, 1, 0]);

    let fan = primitive_geometry(MeshMode::TriangleFan, &FAN, None, true);
    assert_eq!(fan.topology(), GeometryTopology::Triangles);
    assert_eq!(fan.indices(), [1, 2, 0, 2, 3, 0]);

    let indexed_fan = primitive_geometry(MeshMode::TriangleFan, &FAN, Some(&[3, 0, 1, 2]), true);
    assert_eq!(indexed_fan.indices(), [0, 1, 3, 1, 2, 3]);
}

#[test]
fn strip_winding_alternation_gives_every_computed_normal_the_same_side() {
    // Without NORMAL the importer computes one flat normal per triangle. If
    // the odd strip triangles kept the (i, i+1, i+2) order their normals
    // would point the other way.
    for (mode, positions) in [
        (MeshMode::TriangleStrip, RIBBON.as_slice()),
        (MeshMode::TriangleFan, FAN.as_slice()),
    ] {
        let geometry = primitive_geometry(mode, positions, None, false);
        let triangles = positions.len() - 2;
        assert_eq!(
            geometry.vertices().len(),
            triangles * 3,
            "{mode:?}: flat shading owns one vertex per triangle corner",
        );
        for vertex in geometry.vertices() {
            assert_eq!(
                vertex.normal,
                Vec3::Z,
                "{mode:?}: every triangle faces +Z like the first one",
            );
        }
    }
}

#[test]
fn strips_drop_the_degenerate_triangles_that_stitch_them_together() {
    // Exporters join strips by repeating indices. Triangles that repeat an
    // index have no area; they are stitching, not geometry.
    let stitched = primitive_geometry(
        MeshMode::TriangleStrip,
        &RIBBON,
        Some(&[0, 1, 2, 2, 3, 3, 4, 2]),
        true,
    );
    // Candidates: (0,1,2) (1,2,2) (2,2,3) (2,3,3) (3,3,4) (3,2,4).
    assert_eq!(stitched.indices(), [0, 1, 2, 3, 2, 4]);

    // A strip made only of stitching keeps its triangles, so the primitive
    // still loads and owns its vertices.
    let only_stitching =
        primitive_geometry(MeshMode::TriangleStrip, &RIBBON, Some(&[1, 1, 1, 1]), true);
    assert_eq!(only_stitching.indices(), [1, 1, 1, 1, 1, 1]);
}

#[test]
fn line_strips_and_loops_import_as_line_lists() {
    let strip = primitive_geometry(MeshMode::LineStrip, &FAN, None, false);
    assert_eq!(strip.topology(), GeometryTopology::Lines);
    assert_eq!(strip.indices(), [0, 1, 1, 2, 2, 3]);

    let closed = primitive_geometry(MeshMode::LineLoop, &FAN, None, false);
    assert_eq!(closed.topology(), GeometryTopology::Lines);
    assert_eq!(closed.indices(), [0, 1, 1, 2, 2, 3, 3, 0]);

    let indexed_loop = primitive_geometry(MeshMode::LineLoop, &FAN, Some(&[2, 0, 1]), false);
    assert_eq!(indexed_loop.indices(), [2, 0, 0, 1, 1, 2]);

    let indexed_strip = primitive_geometry(MeshMode::LineStrip, &FAN, Some(&[3, 1]), false);
    assert_eq!(indexed_strip.indices(), [3, 1]);
}

#[test]
fn strips_fans_and_loops_that_are_too_short_fail_with_a_structured_error() {
    const TWO: &[u16] = &[0, 1];
    const ONE: &[u16] = &[0];
    for (mode, name, indices, minimum) in [
        (MeshMode::TriangleStrip, "TRIANGLE_STRIP", TWO, "3"),
        (MeshMode::TriangleFan, "TRIANGLE_FAN", TWO, "3"),
        (MeshMode::LineStrip, "LINE_STRIP", ONE, "2"),
        (MeshMode::LineLoop, "LINE_LOOP", ONE, "2"),
    ] {
        let document = primitive_document(mode, &FAN, Some(indices), true);
        let assets = assets_with([("short.gltf", document)]);
        let error = load(&assets, "short.gltf").expect_err("a primitive too short to draw fails");
        assert!(
            matches!(
                &error,
                AssetError::Parse { reason, .. }
                    if reason.contains("mesh 0 primitive 0")
                        && reason.contains(name)
                        && reason.contains(&format!("at least {minimum}"))
            ),
            "{name}: the error must name the primitive, the mode and the minimum: {error:?}",
        );
    }
}

#[test]
fn point_primitives_are_rejected_with_a_structured_error() {
    let document = primitive_document(MeshMode::Points, &FAN, None, true);
    let assets = assets_with([("points.gltf", document)]);
    let error = load(&assets, "points.gltf").expect_err("point primitives are not drawn");
    assert!(
        matches!(
            &error,
            AssetError::Parse { reason, .. }
                if reason.contains("mesh 0 primitive 0") && reason.contains("POINTS")
        ),
        "the error must name the primitive and the mode: {error:?}",
    );
}

#[test]
fn expanded_strip_indices_count_against_the_accessor_byte_budget() {
    // A strip of n indices becomes 3 * (n - 2) list indices: 3,002 two-byte
    // indices turn into 36,000 bytes of triangle list.
    let indices = [0u16, 1, 2].repeat(1_001);
    let document = primitive_document(
        MeshMode::TriangleStrip,
        &TRIANGLE,
        Some(&indices[..3_002]),
        true,
    );
    let assets = assets_with([("long-strip.gltf", document)]);
    load(&assets, "long-strip.gltf").expect("the default budget admits the strip");
    let error = pollster::block_on(assets.load_scene_with_options(
        memory_path("long-strip.gltf"),
        AssetLoadOptions::default().with_max_accessor_bytes(16_384),
    ))
    .expect_err("the expanded triangle list must count against the budget");
    assert!(
        matches!(
            &error,
            AssetError::PolicyViolation { reason, .. }
                if reason.contains("TRIANGLE_STRIP") && reason.contains("36000 bytes")
        ),
        "the budget must run out on the expanded strip: {error:?}",
    );
}

/// glTF primitive modes, by their `mode` value.
#[derive(Debug, Clone, Copy)]
enum MeshMode {
    Points = 0,
    LineLoop = 2,
    LineStrip = 3,
    TriangleStrip = 5,
    TriangleFan = 6,
}

/// A zig-zag ribbon in the XY plane whose strip triangles all face +Z.
const RIBBON: [[f32; 3]; 5] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 2.0, 0.0],
];

/// A unit square in the XY plane, counter-clockwise from the origin.
const FAN: [[f32; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
];

fn primitive_document(
    mode: MeshMode,
    positions: &[[f32; 3]],
    indices: Option<&[u16]>,
    with_normals: bool,
) -> Value {
    let mut buffers = Buffers::default();
    let mut primitive = json!({
        "mode": mode as u32,
        "attributes": {"POSITION": buffers.positions(positions)},
    });
    if with_normals {
        primitive["attributes"]["NORMAL"] =
            json!(buffers.vec3(&vec![[0.0, 0.0, 1.0]; positions.len()]));
    }
    if let Some(indices) = indices {
        primitive["indices"] = json!(buffers.indices(indices));
    }
    buffers.finish(json!({
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [primitive]}],
    }))
}

fn primitive_geometry(
    mode: MeshMode,
    positions: &[[f32; 3]],
    indices: Option<&[u16]>,
    with_normals: bool,
) -> GeometryDesc {
    let document = primitive_document(mode, positions, indices, with_normals);
    let assets = assets_with([("primitive.gltf", document)]);
    let scene_asset = load(&assets, "primitive.gltf")
        .unwrap_or_else(|error| panic!("{mode:?} primitive must load: {error:?}"));
    let mesh = scene_asset.nodes()[0].mesh().expect("the node has a mesh");
    assert_eq!(
        mesh.topology(),
        assets
            .geometry(mesh.geometry())
            .expect("geometry resolves")
            .topology(),
    );
    assets.geometry(mesh.geometry()).expect("geometry resolves")
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
