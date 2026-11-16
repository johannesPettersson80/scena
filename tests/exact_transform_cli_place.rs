//! `scena place` must solve its verbs against exact world matrices.
//!
//! For `--node`, the placed node hangs below `arm`, a rotated node under
//! `rig`, which is rotated and non-uniformly scaled, so its parent is sheared
//! in the world. Each result is checked by carrying the node's box through
//! the plain matrix product of the chain and the transform the command
//! returned.
//!
//! For `--import`, the two anchors that `align_to_anchor` mates each sit on a
//! rotated node under a non-uniformly scaled node of their asset, so both
//! anchor frames are sheared. The result is checked against the matrix
//! product of each import transform and its asset's node chain.
//!
//! The command rounds transforms to three decimals, which bounds the
//! tolerances.
#![cfg(feature = "agent")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use scena::{Mat4, Quat, Transform, Vec3};
use serde_json::json;

const BOX_HALF_SIZE: Vec3 = Vec3::new(0.15, 0.1, 0.05);
/// Three-decimal rounding of translation, rotation, and scale, carried
/// through a parent that scales by up to 2.5.
const TOLERANCE: f32 = 0.012;

fn rig() -> Transform {
    Transform::at(Vec3::new(0.3, 0.4, -0.2))
        .rotate_y_deg(25.0)
        .with_scale(Vec3::new(2.5, 1.0, 0.6))
}

fn arm() -> Transform {
    Transform::at(Vec3::new(0.2, 0.1, 0.0))
        .rotate_x_deg(20.0)
        .rotate_z_deg(40.0)
}

fn part() -> Transform {
    Transform::at(Vec3::new(0.1, 0.3, -0.2))
        .rotate_y_deg(-30.0)
        .rotate_x_deg(15.0)
        .with_scale(Vec3::new(1.2, 0.8, 1.5))
}

#[test]
fn the_placed_node_has_a_sheared_parent() {
    let parent = matrix_of(rig()) * matrix_of(arm());
    let x = parent.transform_vector3(Vec3::X).normalize();
    let y = parent.transform_vector3(Vec3::Y).normalize();
    assert!(
        x.dot(y).abs() > 0.2,
        "the fixture proves nothing unless the arm is sheared: {x:?} {y:?}"
    );
}

#[test]
fn ground_rests_a_node_under_a_sheared_parent_on_the_plane() {
    let placed = place(&["--verb", "ground", "--ground-y", "0"]);
    let (min, _max) = world_bounds(placed);
    assert!(
        min.y.abs() <= TOLERANCE,
        "the grounded node's lowest point must sit on y=0, got {}",
        min.y
    );
    assert_kept_orientation_and_scale(placed);
}

#[test]
fn center_puts_a_node_under_a_sheared_parent_on_the_target() {
    let target = Vec3::new(0.5, 0.25, -0.5);
    let placed = place(&["--verb", "center", "--target", "0.5,0.25,-0.5"]);
    let (min, max) = world_bounds(placed);
    let centre = (min + max) * 0.5;
    assert!(
        (centre - target).length() <= TOLERANCE,
        "the centred node's bounds must be centred on {target:?}, got {centre:?}"
    );
    assert_kept_orientation_and_scale(placed);
}

#[test]
fn fit_to_size_shrinks_a_node_under_a_sheared_parent_to_the_limit() {
    let (before_min, before_max) = world_bounds(part());
    assert!(
        (before_max - before_min).max_element() > 0.3,
        "the fixture must start larger than the limit"
    );
    let placed = place(&["--verb", "fit-to-size", "--max-size", "0.3"]);
    let (min, max) = world_bounds(placed);
    let largest = (max - min).max_element();
    // Only the three-decimal rounding of the scale and rotation separates the
    // result from the limit, and the node's own extent is small.
    assert!(
        (largest - 0.3).abs() <= 0.003,
        "the fitted node's largest extent must be 0.3, got {largest}"
    );
}

#[test]
fn look_at_turns_a_node_under_a_sheared_parent_to_face_the_target() {
    let target = Vec3::new(-0.6, 1.4, 0.8);
    let placed = place(&["--verb", "look_at", "--target", "-0.6,1.4,0.8"]);
    let before = world_matrix(part()).transform_point3(Vec3::ZERO);
    let world = world_matrix(placed);
    let position = world.transform_point3(Vec3::ZERO);
    assert!(
        (position - before).length() <= TOLERANCE,
        "look_at must keep the node where it is: before={before:?}, after={position:?}"
    );
    let forward = world.transform_vector3(Vec3::NEG_Z).normalize();
    let wanted = (target - position).normalize();
    let off_by = forward.angle_between(wanted).to_degrees();
    assert!(
        off_by <= 0.5,
        "the node's -Z axis must point at the target: off by {off_by} degrees"
    );
}

#[test]
fn align_to_anchor_mates_anchors_that_their_assets_shear() {
    let root = unique_temp_dir();
    fs::create_dir(&root).expect("placement temp directory creates");
    let source_asset = sheared_anchor_asset([1.5, 1.0, 0.7], 30.0, "grip");
    let target_asset = sheared_anchor_asset([2.0, 1.0, 0.6], -40.0, "mount");
    fs::write(root.join("source.gltf"), &source_asset).expect("source asset writes");
    fs::write(root.join("target.gltf"), &target_asset).expect("target asset writes");
    let source_import = Transform::at(Vec3::new(-0.8, 0.1, 0.3))
        .rotate_y_deg(15.0)
        .scale_by(1.25);
    let target_import = Transform::at(Vec3::new(0.9, 0.4, -0.5))
        .rotate_x_deg(-20.0)
        .rotate_y_deg(35.0);
    let recipe = root.join("anchors.recipe.json");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "source", "uri": "source.gltf", "transform": raw(source_import) },
                { "id": "target", "uri": "target.gltf", "transform": raw(target_import) }
            ]
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");

    let output = run(&[
        "place",
        path(&recipe),
        "--import",
        "source",
        "--verb",
        "align_to_anchor",
        "--source-anchor",
        "grip",
        "--target-import",
        "target",
        "--target-anchor",
        "mount",
    ]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let result = stdout_json(&output);
    fs::remove_dir_all(root).expect("placement temp directory removes");
    assert_eq!(result["ok"], true, "{result:#}");
    let placed = transform_of(&result);

    let source_anchor = matrix_of(placed) * anchor_in_asset(&source_asset, "grip");
    let target_anchor = matrix_of(target_import) * anchor_in_asset(&target_asset, "mount");
    for (frame, name) in [(source_anchor, "source"), (target_anchor, "target")] {
        let x = frame.transform_vector3(Vec3::X).normalize();
        let y = frame.transform_vector3(Vec3::Y).normalize();
        assert!(
            x.dot(y).abs() > 0.15,
            "the fixture proves nothing unless the {name} anchor is sheared: {x:?} {y:?}"
        );
    }

    let origin = source_anchor.transform_point3(Vec3::ZERO);
    let wanted_origin = target_anchor.transform_point3(Vec3::ZERO);
    assert!(
        (origin - wanted_origin).length() <= 0.006,
        "anchor origin: expected {wanted_origin:?}, got {origin:?}"
    );
    let x_axis = source_anchor.transform_vector3(Vec3::X).normalize();
    let wanted_x = target_anchor.transform_vector3(Vec3::X).normalize();
    assert!(
        x_axis.angle_between(wanted_x).to_degrees() <= 0.4,
        "anchor x axis: expected {wanted_x:?}, got {x_axis:?}"
    );
    let up = up_across(source_anchor);
    let wanted_up = up_across(target_anchor);
    assert!(
        up.angle_between(wanted_up).to_degrees() <= 0.4,
        "anchor up axis: expected {wanted_up:?}, got {up:?}"
    );
    assert!(
        (placed.scale - source_import.scale).abs().max_element() <= 1.0e-3,
        "the import keeps its scale: {:?}",
        placed.scale
    );
}

/// An asset whose anchor sits on a rotated node under a non-uniformly scaled
/// node, two levels below the asset root.
fn sheared_anchor_asset(scale: [f32; 3], turn_degrees: f32, anchor: &str) -> String {
    let (sin, cos) = (turn_degrees.to_radians() * 0.5).sin_cos();
    let (anchor_sin, anchor_cos) = (25.0_f32.to_radians() * 0.5).sin_cos();
    json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [
            { "name": "Root", "children": [1] },
            { "name": "Scaled", "scale": scale, "children": [2] },
            {
                "name": "Rotated",
                "translation": [0.2, 0.1, -0.1],
                "rotation": [0.0, 0.0, sin, cos],
                "extras": { "scena": { "anchors": [{
                    "name": anchor,
                    "translation": [0.1, 0.05, 0.0],
                    "rotation": [0.0, anchor_sin, 0.0, anchor_cos]
                }] } }
            }
        ]
    })
    .to_string()
}

/// The exact frame of an anchor relative to its asset's root: the world
/// matrix of the node that declares it, in a scene that holds only the asset,
/// times the anchor's own transform.
fn anchor_in_asset(gltf: &str, name: &str) -> Mat4 {
    let assets = scena::Assets::new();
    let asset = pollster::block_on(assets.load_scene_from_bytes("anchor.gltf", gltf.as_bytes()))
        .expect("anchor asset loads");
    let mut scene = scena::Scene::new();
    let import = scene
        .instantiate(&asset)
        .expect("anchor asset instantiates");
    let anchor = import.anchor(name).expect("asset declares the anchor");
    scene
        .world_matrix(anchor.node())
        .expect("anchor host resolves")
        * matrix_of(anchor.transform())
}

/// The Y axis of a frame made perpendicular to its X axis.
fn up_across(frame: Mat4) -> Vec3 {
    let x = frame.transform_vector3(Vec3::X).normalize();
    let y = frame.transform_vector3(Vec3::Y);
    (y - x * y.dot(x)).normalize()
}

/// Runs `scena place` on the node below the sheared arm and returns the local
/// transform it proposes.
fn place(verb_args: &[&str]) -> Transform {
    let root = unique_temp_dir();
    fs::create_dir(&root).expect("placement temp directory creates");
    let recipe = root.join("sheared.recipe.json");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{
                "id": "box",
                "primitive": {
                    "kind": "box",
                    "size": [
                        BOX_HALF_SIZE.x * 2.0,
                        BOX_HALF_SIZE.y * 2.0,
                        BOX_HALF_SIZE.z * 2.0
                    ]
                }
            }],
            "materials": [{ "id": "mat", "kind": "unlit", "base_color": "#ffffff" }],
            "nodes": [
                { "id": "rig", "transform": raw(rig()) },
                { "id": "arm", "parent": "rig", "transform": raw(arm()) },
                {
                    "id": "part",
                    "parent": "arm",
                    "geometry": "box",
                    "material": "mat",
                    "transform": raw(part())
                }
            ]
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");

    let mut args = vec!["place", path(&recipe), "--node", "part"];
    args.extend_from_slice(verb_args);
    let output = run(&args);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let result = stdout_json(&output);
    fs::remove_dir_all(root).expect("placement temp directory removes");
    assert_eq!(result["ok"], true, "{result:#}");
    transform_of(&result)
}

/// The transform a placement result proposes.
fn transform_of(result: &serde_json::Value) -> Transform {
    let numbers = |key: &str| -> Vec<f32> {
        result["transform"][key]
            .as_array()
            .unwrap_or_else(|| panic!("transform.{key} is an array: {result:#}"))
            .iter()
            .map(|value| value.as_f64().expect("transform component is a number") as f32)
            .collect()
    };
    let translation = numbers("translation");
    let rotation = numbers("rotation");
    let scale = numbers("scale");
    Transform {
        translation: Vec3::new(translation[0], translation[1], translation[2]),
        rotation: Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]).normalize(),
        scale: Vec3::new(scale[0], scale[1], scale[2]),
    }
}

/// Verbs that only move a node must hand its rotation and scale back.
fn assert_kept_orientation_and_scale(placed: Transform) {
    let original = part();
    assert!(
        placed.rotation.dot(original.rotation).abs() >= 1.0 - 1.0e-5,
        "rotation must be kept: {:?} vs {:?}",
        placed.rotation,
        original.rotation
    );
    assert!(
        (placed.scale - original.scale).abs().max_element() <= 1.0e-3,
        "scale must be kept: {:?} vs {:?}",
        placed.scale,
        original.scale
    );
}

fn raw(transform: Transform) -> serde_json::Value {
    json!({
        "kind": "raw",
        "translation": transform.translation.to_array(),
        "rotation": transform.rotation.to_array(),
        "scale": transform.scale.to_array()
    })
}

fn matrix_of(local: Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}

fn world_matrix(part_local: Transform) -> Mat4 {
    matrix_of(rig()) * matrix_of(arm()) * matrix_of(part_local)
}

/// World bounds of the node's box, corner by corner through the matrix.
fn world_bounds(part_local: Transform) -> (Vec3, Vec3) {
    let world = world_matrix(part_local);
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for corner in 0..8 {
        let sign = |bit: u32| if corner & bit == 0 { -1.0 } else { 1.0 };
        let point = world.transform_point3(BOX_HALF_SIZE * Vec3::new(sign(1), sign(2), sign(4)));
        min = min.min(point);
        max = max.max(point);
    }
    (min, max)
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .expect("scena command runs")
}

fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(output)
        )
    })
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test path is UTF-8")
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    // Tests in this file run in parallel and may share a clock tick.
    static NEXT: AtomicU32 = AtomicU32::new(0);
    std::env::temp_dir().join(format!(
        "scena-exact-place-{}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
