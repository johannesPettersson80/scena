//! Recipe placement verbs must land a node where the verb says even when the
//! node's parent is sheared in the world.
//!
//! Every node under test hangs below `arm`, a rotated node under `rig`, which
//! is rotated and non-uniformly scaled. Each expectation is read back through
//! the exact world matrix: `Scene::world_matrix`, or the world bounds the
//! scene derives from it.
#![cfg(feature = "scene-host")]

use scena::{Aabb, Mat4, NodeKey, Scene, Transform, Vec3};
use serde_json::json;

const IMPORT_TRANSLATION: [f64; 3] = [0.6, 0.2, -0.3];
const IMPORT_ROTATION_DEGREES: [f64; 3] = [0.0, 35.0, 20.0];
const IMPORT_SCALE: [f64; 3] = [2.0, 1.0, 0.5];
const ANCHOR_ASSET: &str = "tests/assets/gltf/anchored_triangle_scene.gltf";

fn recipe() -> serde_json::Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "anchor_asset",
            "uri": ANCHOR_ASSET,
            "transform": {
                "kind": "trs",
                "translation": IMPORT_TRANSLATION,
                "rotation_degrees": IMPORT_ROTATION_DEGREES,
                "scale": IMPORT_SCALE
            }
        }],
        "colors": { "base": "#7097C8" },
        "geometries": [
            { "id": "support_geo", "primitive": { "kind": "box", "size": [0.5, 0.1, 0.4] } },
            { "id": "part_geo", "primitive": { "kind": "box", "size": [0.3, 0.2, 0.1] } },
            { "id": "large_geo", "primitive": { "kind": "box", "size": [2.0, 1.0, 0.5] } }
        ],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "base" }],
        "nodes": [
            {
                "id": "rig",
                "tags": ["rig"],
                "transform": {
                    "kind": "trs",
                    "translation": [0.3, 0.4, -0.2],
                    "rotation_degrees": [0.0, 25.0, 0.0],
                    "scale": [2.5, 1.0, 0.6]
                }
            },
            {
                "id": "arm",
                "parent": "rig",
                "tags": ["arm"],
                "transform": {
                    "kind": "trs",
                    "translation": [0.2, 0.1, 0.0],
                    "rotation_degrees": [20.0, 0.0, 40.0]
                }
            },
            {
                "id": "support",
                "geometry": "support_geo",
                "material": "mat",
                "transform": { "kind": "trs", "translation": [-0.4, -0.5, 0.3] }
            },
            {
                "id": "centered",
                "parent": "arm",
                "geometry": "part_geo",
                "material": "mat",
                "transform": { "kind": "center" }
            },
            {
                "id": "grounded",
                "parent": "arm",
                "geometry": "part_geo",
                "material": "mat",
                "transform": { "kind": "ground", "plane_y": -0.25 }
            },
            {
                "id": "placed",
                "parent": "arm",
                "geometry": "part_geo",
                "material": "mat",
                "transform": { "kind": "place_on", "target": "support", "offset": [0.0, 0.02, 0.0] }
            },
            {
                "id": "fit",
                "parent": "arm",
                "geometry": "large_geo",
                "material": "mat",
                "transform": { "kind": "fit_to_size", "size": [0.4, 0.4, 0.4] }
            },
            {
                "id": "looker",
                "parent": "arm",
                "geometry": "part_geo",
                "material": "mat",
                "tags": ["looker"],
                "transform": {
                    "kind": "look_at",
                    "eye": [0.5, 0.8, 1.0],
                    "target": [-0.2, 0.1, 0.0]
                }
            },
            {
                "id": "aligned",
                "parent": "arm",
                "geometry": "part_geo",
                "material": "mat",
                "tags": ["aligned"],
                "transform": { "kind": "align_to_anchor", "anchor": "anchor_asset.mount" }
            }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [2.0, 2.0, 3.0], "target": [0.0, 0.0, 0.0] }
        }],
        "capture": { "width": 64, "height": 64 }
    })
}

struct Built {
    host: scena::SceneHostCore<scena::DefaultAssetFetcher>,
    handles: Vec<(String, u64)>,
}

impl Built {
    fn new() -> Self {
        let text = serde_json::to_string_pretty(&recipe()).expect("recipe serializes");
        let validation = scena::validate_scene_recipe_json(&text);
        assert!(validation.ok, "recipe should validate: {validation:#?}");
        let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
            "tests/assets/exact_transform.recipe.json",
            &text,
            scena::RecipeBuildPolicy::testing(),
        ))
        .expect("recipe build succeeds");
        assert!(build.manifest.ok, "{:#?}", build.manifest);
        let handles = build
            .manifest
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.handle))
            .collect();
        Self {
            host: build.host,
            handles,
        }
    }

    fn scene(&self) -> &Scene {
        self.host.scene()
    }

    fn bounds(&self, id: &str) -> Aabb {
        let handle = self
            .handles
            .iter()
            .find(|(node, _)| node == id)
            .map(|(_, handle)| *handle)
            .unwrap_or_else(|| panic!("node {id} is in the build manifest"));
        self.host
            .node_world_bounds(handle)
            .expect("bounds query succeeds")
            .unwrap_or_else(|| panic!("node {id} has bounds"))
    }

    fn tagged(&self, tag: &str) -> NodeKey {
        let mut nodes = self.scene().tagged(tag);
        let node = nodes
            .next()
            .unwrap_or_else(|| panic!("a node tagged {tag}"));
        assert!(nodes.next().is_none(), "tag {tag} names one node");
        node
    }

    fn world_matrix(&self, tag: &str) -> Mat4 {
        self.scene()
            .world_matrix(self.tagged(tag))
            .expect("tagged node resolves")
    }
}

fn assert_near(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 2.0e-4,
        "{what}: expected {expected}, got {actual}"
    );
}

fn assert_vec3_near(actual: Vec3, expected: Vec3, what: &str) {
    assert!(
        (actual - expected).length() <= 2.0e-4,
        "{what}: expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn the_parent_of_every_placed_node_is_sheared() {
    let built = Built::new();
    let arm = built.world_matrix("arm");
    let x = arm.transform_vector3(Vec3::X).normalize();
    let y = arm.transform_vector3(Vec3::Y).normalize();
    assert!(
        x.dot(y).abs() > 0.2,
        "the fixture proves nothing unless the arm is sheared: {x:?} {y:?}"
    );
}

#[test]
fn center_puts_the_world_bounds_centre_on_the_origin_under_a_sheared_parent() {
    let built = Built::new();
    assert_vec3_near(
        built.bounds("centered").center(),
        Vec3::ZERO,
        "centre of the centred node",
    );
}

#[test]
fn ground_rests_the_world_bounds_on_the_plane_under_a_sheared_parent() {
    let built = Built::new();
    assert_near(
        built.bounds("grounded").min.y,
        -0.25,
        "lowest point of the grounded node",
    );
}

#[test]
fn place_on_rests_the_node_on_its_target_under_a_sheared_parent() {
    let built = Built::new();
    let support = built.bounds("support");
    let placed = built.bounds("placed");

    assert_near(
        placed.min.y,
        support.max.y + 0.02,
        "underside of the placed node",
    );
    assert_near(
        placed.center().x,
        support.center().x,
        "placed node centre x",
    );
    assert_near(
        placed.center().z,
        support.center().z,
        "placed node centre z",
    );
}

#[test]
fn fit_to_size_fills_the_requested_box_under_a_sheared_parent() {
    let built = Built::new();
    let fit = built.bounds("fit");
    let fill = (fit.max - fit.min) / Vec3::splat(0.4);

    assert_near(
        fill.max_element(),
        1.0,
        "largest fill ratio of the fitted node",
    );
}

#[test]
fn look_at_puts_the_node_at_the_eye_facing_the_target_under_a_sheared_parent() {
    let built = Built::new();
    let world = built.world_matrix("looker");
    let eye = Vec3::new(0.5, 0.8, 1.0);
    let target = Vec3::new(-0.2, 0.1, 0.0);

    assert_vec3_near(world.transform_point3(Vec3::ZERO), eye, "looker position");
    assert_vec3_near(
        world.transform_vector3(Vec3::NEG_Z).normalize(),
        (target - eye).normalize(),
        "looker viewing direction",
    );
    // The node's Y axis lies in the plane of the viewing direction and the
    // default up, on the up side.
    let plane_normal = (target - eye).cross(Vec3::Y).normalize();
    let up = world.transform_vector3(Vec3::Y).normalize();
    assert_near(
        up.dot(plane_normal),
        0.0,
        "looker up axis out of the up plane",
    );
    assert!(up.y > 0.0, "looker up axis must point up: {up:?}");
}

#[test]
fn align_to_anchor_matches_the_anchor_frame_as_far_as_a_sheared_parent_allows() {
    let built = Built::new();
    let anchor = anchor_world_matrix();
    let aligned = built.world_matrix("aligned");
    let anchor_x = anchor.transform_vector3(Vec3::X);
    let anchor_y = anchor.transform_vector3(Vec3::Y);

    assert_vec3_near(
        aligned.transform_point3(Vec3::ZERO),
        anchor.transform_point3(Vec3::ZERO),
        "aligned node origin",
    );
    assert_vec3_near(
        aligned.transform_vector3(Vec3::X).normalize(),
        anchor_x.normalize(),
        "aligned node x axis",
    );
    for (axis, name) in [(Vec3::X, "x"), (Vec3::Y, "y"), (Vec3::Z, "z")] {
        assert_near(
            aligned.transform_vector3(axis).length(),
            anchor.transform_vector3(axis).length(),
            &format!("aligned node {name} axis length"),
        );
    }
    let plane_normal = anchor_x.cross(anchor_y).normalize();
    let aligned_y = aligned.transform_vector3(Vec3::Y).normalize();
    assert_near(
        aligned_y.dot(plane_normal),
        0.0,
        "aligned node y axis out of the anchor's x-y plane",
    );
    assert!(aligned_y.dot(anchor_y) > 0.0);
}

/// The anchor's exact world matrix, from the recipe's import transform and
/// the anchor frame the asset declares on its root node.
fn anchor_world_matrix() -> Mat4 {
    let assets = scena::Assets::new();
    let asset = pollster::block_on(assets.load_scene(ANCHOR_ASSET)).expect("anchor asset loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&asset)
        .expect("anchor asset instantiates");
    let anchor = import.anchor("mount").expect("asset declares the anchor");
    let host = scene
        .world_matrix(anchor.node())
        .expect("anchor host resolves");
    let narrow = |values: [f64; 3]| Vec3::new(values[0] as f32, values[1] as f32, values[2] as f32);
    let degrees = narrow(IMPORT_ROTATION_DEGREES);
    let import_transform = Transform::at(narrow(IMPORT_TRANSLATION))
        .rotate_x_deg(degrees.x)
        .rotate_y_deg(degrees.y)
        .rotate_z_deg(degrees.z)
        .with_scale(narrow(IMPORT_SCALE));
    matrix_of(import_transform) * host * matrix_of(anchor.transform())
}

fn matrix_of(local: Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}
