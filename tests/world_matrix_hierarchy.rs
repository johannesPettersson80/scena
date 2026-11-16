//! Scene-graph world transforms are affine matrices.
//!
//! A parent with non-uniform or negative scale and a rotated child shears, and
//! translation/rotation/scale cannot represent shear. These tests pin the exact
//! world matrix, the TRS view kept for compatibility, and a glTF import whose
//! hierarchy needs the matrix.
#![cfg(not(target_arch = "wasm32"))]

use std::f32::consts::FRAC_PI_2;

use base64::Engine as _;
use scena::{
    Aabb, Assets, Color, ExplodedView, GeometryDesc, Mat4, MaterialDesc, NodeKey,
    PerspectiveCamera, Quat, Scene, SceneSkinBinding, SkinningMatrix, Transform, Vec3,
};

const TOLERANCE: f32 = 1.0e-5;

#[test]
fn world_matrix_keeps_shear_from_non_uniform_parent_scale_and_rotated_child() {
    let chain = [
        Transform::IDENTITY.with_scale(Vec3::new(2.0, 1.0, 1.0)),
        Transform {
            rotation: Quat::from_rotation_z(FRAC_PI_2),
            ..Transform::IDENTITY
        },
    ];
    let mut scene = Scene::new();
    let nodes = add_chain(&mut scene, &chain);

    let world = scene.world_matrix(nodes[1]).expect("child resolves");

    assert_point_near(
        "child-local +X",
        world.transform_point3(Vec3::X),
        Vec3::new(0.0, 1.0, 0.0),
    );
    assert_point_near(
        "child-local +Y",
        world.transform_point3(Vec3::Y),
        Vec3::new(-2.0, 0.0, 0.0),
    );
    assert_matrix_near(world, ground_truth(&chain));
}

#[test]
fn world_matrix_mirrors_a_rotated_and_translated_child_under_a_mirrored_parent() {
    let chain = [
        Transform::at(Vec3::new(0.5, -0.25, 1.0)).with_scale(Vec3::new(-1.0, 1.0, 1.0)),
        Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_z(30.0_f32.to_radians()),
            scale: Vec3::ONE,
        },
    ];
    let mut scene = Scene::new();
    let nodes = add_chain(&mut scene, &chain);

    let world = scene.world_matrix(nodes[1]).expect("child resolves");

    // The parent mirrors X, so the child origin lands at x = 0.5 - 1.0 and its
    // +30 degree turn reads as -30 degrees in the world.
    let (sin, cos) = 30.0_f32.to_radians().sin_cos();
    assert_point_near(
        "child origin",
        world.transform_point3(Vec3::ZERO),
        Vec3::new(-0.5, 1.75, 4.0),
    );
    assert_point_near(
        "child-local +X direction",
        world.transform_vector3(Vec3::X),
        Vec3::new(-cos, sin, 0.0),
    );
    assert_point_near(
        "child-local +Y direction",
        world.transform_vector3(Vec3::Y),
        Vec3::new(sin, cos, 0.0),
    );
    assert!(
        world.determinant() < 0.0,
        "a mirrored hierarchy must keep its negative determinant"
    );
    assert_matrix_near(world, ground_truth(&chain));
}

#[test]
fn world_matrix_is_the_matrix_product_of_a_three_level_chain() {
    let chain = [
        Transform {
            translation: Vec3::new(1.0, -2.0, 0.5),
            rotation: Quat::from_rotation_y(0.7),
            scale: Vec3::new(1.5, 0.5, 2.0),
        },
        Transform {
            translation: Vec3::new(-0.25, 0.75, 1.25),
            rotation: Quat::from_rotation_x(-1.1),
            scale: Vec3::new(0.8, 3.0, -1.2),
        },
        Transform {
            translation: Vec3::new(2.0, 0.1, -0.6),
            rotation: Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0).normalize(), 2.3),
            scale: Vec3::new(1.0, 1.0, 0.25),
        },
    ];
    let mut scene = Scene::new();
    let nodes = add_chain(&mut scene, &chain);

    for depth in 1..=chain.len() {
        assert_matrix_near(
            scene.world_matrix(nodes[depth - 1]).expect("node resolves"),
            ground_truth(&chain[..depth]),
        );
    }
    let world = scene.world_matrix(nodes[2]).expect("leaf resolves");
    for probe in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(0.3, -0.7, 1.9)] {
        let expected = chain.iter().rev().fold(probe, |point, local| {
            matrix_of(*local).transform_point3(point)
        });
        assert_point_near("leaf-local probe", world.transform_point3(probe), expected);
    }
}

#[test]
fn world_matrix_follows_ancestor_edits_and_disappears_with_the_node() {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("parent inserts");
    let child = scene
        .add_empty(
            parent,
            Transform {
                rotation: Quat::from_rotation_z(FRAC_PI_2),
                ..Transform::IDENTITY
            },
        )
        .expect("child inserts");
    assert_point_near(
        "unscaled parent",
        scene
            .world_matrix(child)
            .expect("child resolves")
            .transform_point3(Vec3::Y),
        Vec3::new(-1.0, 0.0, 0.0),
    );

    scene
        .set_transform(
            parent,
            Transform::IDENTITY.with_scale(Vec3::new(2.0, 1.0, 1.0)),
        )
        .expect("parent rescales");
    assert_point_near(
        "rescaled parent",
        scene
            .world_matrix(child)
            .expect("child resolves")
            .transform_point3(Vec3::Y),
        Vec3::new(-2.0, 0.0, 0.0),
    );

    scene.remove_node(child).expect("child removes");
    assert_eq!(scene.world_matrix(child), None);
}

#[test]
fn world_transform_stays_the_composed_trs_when_the_hierarchy_cannot_shear() {
    // Translation, rotation, and uniform scale only: the TRS view is exact and
    // must stay bit-for-bit what `Transform::compose` has always produced.
    let chain = [
        Transform::at(Vec3::new(1.0, 2.0, 3.0))
            .rotate_y_deg(40.0)
            .with_uniform_scale(2.0),
        Transform::at(Vec3::new(-0.5, 0.25, 4.0))
            .rotate_x_deg(-25.0)
            .with_uniform_scale(0.5),
        Transform::at(Vec3::new(0.0, 1.0, 0.0)).rotate_z_deg(75.0),
    ];
    let mut scene = Scene::new();
    let nodes = add_chain(&mut scene, &chain);

    let mut composed = Transform::IDENTITY;
    for (node, local) in nodes.iter().zip(chain) {
        composed = Transform::compose(composed, local);
        assert_eq!(
            scene.world_transform(*node).expect("node resolves"),
            composed
        );
        assert_matrix_near(
            scene.world_matrix(*node).expect("node resolves"),
            matrix_of(composed),
        );
    }
}

#[test]
fn world_transform_decomposes_a_mirrored_matrix_without_shear_exactly() {
    // A non-uniformly scaled, mirrored parent with a child rotated by a half
    // turn has no shear, so the TRS view must reproduce the matrix.
    let chain = [
        Transform::at(Vec3::new(0.5, 0.0, -1.0)).with_scale(Vec3::new(-2.0, 1.0, 3.0)),
        Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_z(std::f32::consts::PI),
            scale: Vec3::new(1.0, 0.5, 1.0),
        },
    ];
    let mut scene = Scene::new();
    let nodes = add_chain(&mut scene, &chain);

    let world = scene.world_matrix(nodes[1]).expect("child resolves");
    let decomposed = scene.world_transform(nodes[1]).expect("child resolves");

    assert_matrix_near(world, ground_truth(&chain));
    assert_matrix_near(matrix_of(decomposed), world);
}

#[test]
fn world_transform_of_a_sheared_node_keeps_the_exact_origin_and_forward_axis() {
    let chain = [
        Transform::at(Vec3::new(3.0, 0.0, 0.0)).with_scale(Vec3::new(1.0, 1.0, 4.0)),
        Transform {
            translation: Vec3::new(0.0, 1.0, 2.0),
            rotation: Quat::from_rotation_x(0.6),
            scale: Vec3::ONE,
        },
    ];
    let mut scene = Scene::new();
    let nodes = add_chain(&mut scene, &chain);

    let world = ground_truth(&chain);
    assert_matrix_near(scene.world_matrix(nodes[1]).expect("child resolves"), world);
    let decomposed = scene.world_transform(nodes[1]).expect("child resolves");

    // Shear cannot survive a TRS decomposition, but positions and the axis
    // cameras and lights look along must.
    assert_point_near(
        "decomposed origin",
        decomposed.translation,
        world.transform_point3(Vec3::ZERO),
    );
    assert_point_near(
        "decomposed forward axis",
        decomposed.rotation * Vec3::Z,
        world.transform_vector3(Vec3::Z).normalize(),
    );
    assert!(
        (decomposed.rotation.length() - 1.0).abs() <= TOLERANCE,
        "the decomposed rotation must stay a unit quaternion"
    );
}

#[test]
fn gltf_import_places_a_rotated_mesh_under_a_non_uniformly_scaled_parent() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene_from_bytes(
        "non_uniform_parent_rotated_child.gltf",
        non_uniform_parent_gltf().as_bytes(),
    ))
    .expect("inline glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("inline glTF instantiates");
    let bar = import.node("RotatedBar").expect("mesh node resolves");

    // The mesh is a 1.0 x 0.2 bar in its own XY plane. Its node turns it an
    // eighth turn about Z, and the parent then stretches world X by two, which
    // shears the bar: no translation/rotation/scale reproduces this placement.
    let diagonal = std::f32::consts::FRAC_1_SQRT_2;
    let world = scene.world_matrix(bar).expect("mesh node resolves");
    assert_point_near(
        "bar-local +X end",
        world.transform_point3(Vec3::new(0.5, 0.0, 0.0)),
        Vec3::new(diagonal, 0.5 * diagonal, 0.0),
    );
    assert_point_near(
        "bar-local +Y edge",
        world.transform_point3(Vec3::new(0.0, 0.1, 0.0)),
        Vec3::new(-0.2 * diagonal, 0.1 * diagonal, 0.0),
    );

    let expected = Aabb::new(
        Vec3::new(-1.2 * diagonal, -0.6 * diagonal, 0.0),
        Vec3::new(1.2 * diagonal, 0.6 * diagonal, 0.0),
    );
    assert_bounds_near(
        "import bounds",
        import.bounds_world(&scene).expect("import has bounds"),
        expected,
    );
    assert_bounds_near(
        "asset-backed node bounds",
        scene
            .node_world_bounds(bar, &assets)
            .expect("mesh node exists")
            .expect("mesh node has bounds"),
        expected,
    );
}

#[test]
fn move_origin_to_moves_a_sheared_node_without_changing_its_shape() {
    let chain = [
        Transform::at(Vec3::new(0.5, 0.0, -1.0)).with_scale(Vec3::new(2.0, 1.0, 0.5)),
        Transform {
            translation: Vec3::new(0.25, 0.5, 0.0),
            rotation: Quat::from_rotation_z(0.8) * Quat::from_rotation_x(0.3),
            scale: Vec3::new(1.0, 1.5, 1.0),
        },
    ];
    let mut scene = Scene::new();
    let nodes = add_chain(&mut scene, &chain);
    let before = scene.world_matrix(nodes[1]).expect("node resolves");
    let target = Vec3::new(3.0, -2.0, 1.0);

    scene
        .move_origin_to(nodes[1], target)
        .expect("a sheared node moves");

    let after = scene.world_matrix(nodes[1]).expect("node resolves");
    assert_point_near("moved origin", after.transform_point3(Vec3::ZERO), target);
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        assert_point_near(
            "basis axis",
            after.transform_vector3(axis),
            before.transform_vector3(axis),
        );
    }
}

#[test]
fn look_at_point_aims_a_camera_under_a_non_uniformly_scaled_parent() {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(1.0, 0.0, 0.0))
                .rotate_y_deg(30.0)
                .with_scale(Vec3::new(2.0, 1.0, 0.5)),
        )
        .expect("parent inserts");
    let camera = scene
        .add_perspective_camera(
            parent,
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.5, 1.0, 2.0)),
        )
        .expect("camera inserts");
    let camera_node = scene.camera_node(camera).expect("camera has a node");
    let target = Vec3::new(-2.0, 0.5, -3.0);

    scene
        .look_at_point(camera, target)
        .expect("camera aims under a scaled parent");

    let world = scene.world_matrix(camera_node).expect("camera resolves");
    let eye = world.transform_point3(Vec3::ZERO);
    assert_point_near(
        "camera forward axis",
        world.transform_vector3(Vec3::NEG_Z).normalize(),
        (target - eye).normalize(),
    );
    // The view matrix is the inverse of that world matrix, so the target sits
    // on the optical axis and projects to the viewport centre.
    let view = world.inverse().transform_point3(target);
    assert!(view.x.abs() <= 1.0e-4 && view.y.abs() <= 1.0e-4 && view.z < 0.0);
    let projected = scene
        .project_world_point(camera, target, 200, 200)
        .expect("camera resolves")
        .expect("target is in front of the camera");
    assert!(
        (projected.x - 100.0).abs() <= 0.05 && (projected.y - 100.0).abs() <= 0.05,
        "the aimed-at point must project to the centre, got ({}, {})",
        projected.x,
        projected.y
    );
}

#[test]
fn project_world_point_inverts_the_camera_world_matrix() {
    let chain = [
        Transform::at(Vec3::new(0.0, 1.0, 0.0))
            .rotate_y_deg(25.0)
            .with_scale(Vec3::new(1.5, 1.0, 1.0)),
        // Turned about Y as well as X, so the parent's X stretch shears it.
        Transform::at(Vec3::new(0.2, 0.0, 4.0))
            .rotate_y_deg(20.0)
            .rotate_x_deg(-12.0),
    ];
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), chain[0])
        .expect("parent inserts");
    let camera = scene
        .add_perspective_camera(
            parent,
            PerspectiveCamera::default().with_aspect(1.0),
            chain[1],
        )
        .expect("camera inserts");
    let point = Vec3::new(0.4, 0.9, 0.3);

    let view = ground_truth(&chain).inverse().transform_point3(point);
    let focal = (PerspectiveCamera::default().vertical_fov.radians() * 0.5)
        .tan()
        .recip();
    let expected_x = (view.x * focal / -view.z * 0.5 + 0.5) * 200.0;
    let expected_y = (1.0 - (view.y * focal / -view.z * 0.5 + 0.5)) * 200.0;

    let projected = scene
        .project_world_point(camera, point, 200, 200)
        .expect("camera resolves")
        .expect("the point is in front of the camera");
    assert!(
        (projected.x - expected_x).abs() <= 1.0e-2 && (projected.y - expected_y).abs() <= 1.0e-2,
        "expected ({expected_x}, {expected_y}), got ({}, {})",
        projected.x,
        projected.y
    );
    assert!((projected.depth + view.z).abs() <= 1.0e-4);
}

#[test]
fn skin_matrices_follow_joint_world_matrices_under_sheared_ancestors() {
    let mut scene = Scene::new();
    let rig = add_chain(
        &mut scene,
        &[
            Transform::at(Vec3::new(0.0, 1.0, 0.0)).with_scale(Vec3::new(2.0, 1.0, 1.0)),
            Transform {
                translation: Vec3::new(0.3, 0.2, 0.1),
                rotation: Quat::from_rotation_z(0.7),
                scale: Vec3::ONE,
            },
        ],
    );
    let skinned = add_chain(
        &mut scene,
        &[
            Transform::IDENTITY
                .rotate_x_deg(20.0)
                .with_scale(Vec3::new(1.0, 3.0, 1.0)),
            Transform::at(Vec3::new(-0.4, 0.0, 0.6)).rotate_y_deg(50.0),
        ],
    );
    let (joint, mesh) = (rig[1], skinned[1]);
    scene
        .set_skin_binding(
            mesh,
            SceneSkinBinding::new(vec![joint], vec![SkinningMatrix::IDENTITY]),
        )
        .expect("skin binds");

    let matrices = scene.skin_matrices(mesh).expect("skin resolves");
    let expected = scene.world_matrix(mesh).expect("mesh resolves").inverse()
        * scene.world_matrix(joint).expect("joint resolves");

    for probe in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(0.3, -0.7, 1.9)] {
        assert_point_near(
            "skinned probe",
            matrices[0].transform_position(probe),
            expected.transform_point3(probe),
        );
    }
}

#[test]
fn exploded_view_moves_the_parts_of_a_sheared_assembly_without_reshaping_them() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let assembly = scene
        .add_empty(
            scene.root(),
            Transform::IDENTITY.with_scale(Vec3::new(2.0, 1.0, 1.0)),
        )
        .expect("assembly inserts");
    let parts = [
        Transform::at(Vec3::new(-0.5, 0.0, 0.0)).rotate_z_deg(45.0),
        Transform::at(Vec3::new(0.5, 0.3, 0.0)).rotate_y_deg(30.0),
    ]
    .map(|local| {
        scene
            .mesh(geometry, material)
            .parent(assembly)
            .transform(local)
            .add()
            .expect("part inserts")
    });
    let assembly_center = scene
        .node_world_bounds(assembly, &assets)
        .expect("assembly exists")
        .expect("assembly has bounds")
        .center();
    let before = parts.map(|part| {
        let center = scene
            .node_world_bounds(part, &assets)
            .expect("part exists")
            .expect("part has bounds")
            .center();
        (scene.world_matrix(part).expect("part resolves"), center)
    });

    let plan = ExplodedView::from_node(assembly)
        .distance(0.5)
        .transforms(&scene, &assets)
        .expect("a sheared assembly explodes");
    scene
        .set_transforms(&plan.as_transform_updates())
        .expect("plan applies");

    for (part, (world_before, center_before)) in parts.into_iter().zip(before) {
        let world_after = scene.world_matrix(part).expect("part resolves");
        let direction = (center_before - assembly_center).normalize();
        assert_point_near(
            "exploded part origin",
            world_after.transform_point3(Vec3::ZERO),
            world_before.transform_point3(Vec3::ZERO) + direction * 0.5,
        );
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            assert_point_near(
                "exploded part basis axis",
                world_after.transform_vector3(axis),
                world_before.transform_vector3(axis),
            );
        }
    }
}

fn add_chain(scene: &mut Scene, chain: &[Transform]) -> Vec<NodeKey> {
    let mut parent = scene.root();
    chain
        .iter()
        .map(|local| {
            parent = scene.add_empty(parent, *local).expect("chain node inserts");
            parent
        })
        .collect()
}

fn matrix_of(local: Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}

fn ground_truth(chain: &[Transform]) -> Mat4 {
    chain
        .iter()
        .fold(Mat4::IDENTITY, |world, local| world * matrix_of(*local))
}

fn assert_point_near(label: &str, actual: Vec3, expected: Vec3) {
    assert!(
        actual.abs_diff_eq(expected, TOLERANCE),
        "{label}: expected {expected:?}, got {actual:?}"
    );
}

fn assert_matrix_near(actual: Mat4, expected: Mat4) {
    assert!(
        actual.abs_diff_eq(expected, TOLERANCE),
        "expected world matrix {expected:?}, got {actual:?}"
    );
}

fn assert_bounds_near(label: &str, actual: Aabb, expected: Aabb) {
    assert!(
        actual.min.abs_diff_eq(expected.min, TOLERANCE)
            && actual.max.abs_diff_eq(expected.max, TOLERANCE),
        "{label}: expected {expected:?}, got {actual:?}"
    );
}

/// A two-node glTF: a parent scaled (2, 1, 1) and a child mesh node rotated an
/// eighth turn about Z. The mesh is a single-sided 1.0 x 0.2 quad facing +Z.
fn non_uniform_parent_gltf() -> String {
    let positions: [[f32; 3]; 4] = [
        [-0.5, -0.1, 0.0],
        [0.5, -0.1, 0.0],
        [0.5, 0.1, 0.0],
        [-0.5, 0.1, 0.0],
    ];
    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
    let mut buffer = Vec::new();
    for position in positions {
        for component in position {
            buffer.extend_from_slice(&component.to_le_bytes());
        }
    }
    for _ in positions {
        for component in [0.0_f32, 0.0, 1.0] {
            buffer.extend_from_slice(&component.to_le_bytes());
        }
    }
    let index_offset = buffer.len();
    for index in indices {
        buffer.extend_from_slice(&index.to_le_bytes());
    }
    let (sin, cos) = (std::f32::consts::FRAC_PI_8).sin_cos();
    serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [
            { "name": "ScaledParent", "scale": [2.0, 1.0, 1.0], "children": [1] },
            { "name": "RotatedBar", "rotation": [0.0, 0.0, sin, cos], "mesh": 0 }
        ],
        "meshes": [{
            "primitives": [{ "attributes": { "POSITION": 0, "NORMAL": 1 }, "indices": 2 }]
        }],
        "accessors": [
            {
                "bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
                "min": [-0.5, -0.1, 0.0], "max": [0.5, 0.1, 0.0]
            },
            { "bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC3" },
            { "bufferView": 2, "componentType": 5123, "count": 6, "type": "SCALAR" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 48, "target": 34962 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 48, "target": 34962 },
            { "buffer": 0, "byteOffset": index_offset, "byteLength": 12, "target": 34963 }
        ],
        "buffers": [{
            "byteLength": buffer.len(),
            "uri": format!(
                "data:application/octet-stream;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&buffer)
            )
        }]
    })
    .to_string()
}
