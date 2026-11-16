//! Instance sets on the retained prepare path.
//!
//! An instance set draws `node world x instance local`. The retained path used
//! to refresh only the instance half, so moving the node that holds a set (or
//! any ancestor of it) left the instances where the first prepare put them.

use crate::assets::Assets;
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::scene::{DirectionalLight, InstanceId, InstanceSetKey, NodeKey, Scene, Transform, Vec3};

use super::support::{
    Backend, PrepareWork, RetainedFrame, fresh_full_prepare_frame, retained_frame_after,
    visible_centroid,
};

const SIZE: u32 = 64;
/// Rows above this hold the instance set; the witness mesh sits below it.
const INSTANCE_ROWS: u32 = 44;

struct InstanceFixture {
    scene: Scene,
    assets: Assets,
    parent: NodeKey,
    node: NodeKey,
    set: InstanceSetKey,
    first_instance: InstanceId,
}

/// Two boxes stacked vertically, drawn as one instance set under a parent,
/// plus a small witness mesh that keeps the frame non-empty when the set is
/// hidden.
fn instance_fixture() -> InstanceFixture {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_linear_rgb(0.9, 0.6, 0.2),
        0.0,
        0.8,
    ));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    let witness = assets.create_geometry(GeometryDesc::box_xyz(0.12, 0.12, 0.12));
    scene
        .mesh(witness, material)
        .transform(Transform::at(Vec3::new(0.0, -0.75, 0.0)))
        .add()
        .expect("witness mesh inserts");
    let parent = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("parent inserts");
    let (node, set) = scene
        .add_instance_set_node(
            parent,
            geometry,
            material,
            Transform::at(Vec3::new(-0.45, 0.0, 0.0)),
        )
        .expect("instance set inserts");
    let first_instance = scene
        .push_instance(set, Transform::at(Vec3::new(0.0, 0.22, 0.0)))
        .expect("instance inserts");
    scene
        .push_instance(set, Transform::at(Vec3::new(0.0, -0.22, 0.0)))
        .expect("instance inserts");
    InstanceFixture {
        scene,
        assets,
        parent,
        node,
        set,
        first_instance,
    }
}

/// Centroid of the instance set's pixels, ignoring the witness mesh.
fn instance_centroid(frame: &[u8]) -> Option<(f32, f32)> {
    visible_centroid(&frame[..(INSTANCE_ROWS * SIZE * 4) as usize], SIZE)
}

/// Applies `change` after one prepared GPU frame and returns the next frame.
fn retained_gpu_frame(
    fixture: &mut InstanceFixture,
    change: impl FnOnce(&mut Scene),
) -> Option<RetainedFrame> {
    retained_frame_after(
        Backend::Gpu,
        SIZE,
        &mut fixture.scene,
        &fixture.assets,
        change,
    )
}

fn assert_cpu_renderer_agrees(case: &str, fixture: &mut InstanceFixture, gpu: &[u8]) {
    let camera = fixture.scene.active_camera().expect("active camera");
    let cpu = fresh_full_prepare_frame(
        Backend::Cpu,
        SIZE,
        SIZE,
        &mut fixture.scene,
        &fixture.assets,
        camera,
    );
    let gpu = instance_centroid(gpu).expect("the GPU frame shows the instances");
    let cpu = instance_centroid(&cpu).expect("the CPU frame shows the instances");
    assert!(
        (gpu.0 - cpu.0).abs() <= 1.5 && (gpu.1 - cpu.1).abs() <= 1.5,
        "{case}: the GPU and CPU renderers must place the instances together: gpu={gpu:?}, cpu={cpu:?}"
    );
}

#[test]
fn moving_an_instance_set_node_moves_its_instances_on_the_retained_gpu_path() {
    let mut fixture = instance_fixture();
    let node = fixture.node;
    let Some(retained) = retained_gpu_frame(&mut fixture, |scene| {
        scene
            .set_transform(node, Transform::at(Vec3::new(0.45, 0.0, 0.0)))
            .expect("instance set node moves");
    }) else {
        return;
    };

    assert_eq!(
        retained.work,
        PrepareWork::RETAINED,
        "moving an instance set node is a transform-only change"
    );
    let centroid = instance_centroid(&retained.frame).expect("the moved instances are visible");
    assert!(
        centroid.0 > SIZE as f32 * 0.6,
        "the instances must follow their node to the right half of the frame: {centroid:?}"
    );
    retained.assert_matches_full_prepare_within_rounding("moved instance set node");
    assert_cpu_renderer_agrees("moved instance set node", &mut fixture, &retained.frame);
}

#[test]
fn moving_an_ancestor_of_an_instance_set_moves_its_instances_on_the_retained_gpu_path() {
    let mut fixture = instance_fixture();
    // Non-uniform scale on the parent over a rotated child shears the set's
    // world matrix, so this also covers placements no TRS can describe.
    fixture
        .scene
        .set_transform(
            fixture.node,
            Transform::at(Vec3::new(-0.45, 0.0, 0.0)).rotate_z_deg(20.0),
        )
        .expect("instance set node rotates");
    let parent = fixture.parent;
    let Some(retained) = retained_gpu_frame(&mut fixture, |scene| {
        scene
            .set_transform(
                parent,
                Transform::at(Vec3::new(0.7, 0.25, 0.0))
                    .rotate_z_deg(35.0)
                    .with_scale(Vec3::new(1.4, 0.8, 1.0)),
            )
            .expect("parent moves");
    }) else {
        return;
    };

    assert_eq!(retained.work, PrepareWork::RETAINED);
    retained.assert_matches_full_prepare_within_rounding("moved instance set ancestor");
    assert_cpu_renderer_agrees("moved instance set ancestor", &mut fixture, &retained.frame);
}

#[test]
fn moving_one_instance_and_its_set_node_together_composes_both_on_the_retained_gpu_path() {
    let mut fixture = instance_fixture();
    let (node, set, instance) = (fixture.node, fixture.set, fixture.first_instance);
    let Some(retained) = retained_gpu_frame(&mut fixture, |scene| {
        scene
            .set_instance_transform(
                set,
                instance,
                Transform::at(Vec3::new(0.3, 0.3, 0.0)).with_uniform_scale(1.5),
            )
            .expect("instance moves");
        scene
            .set_transform(node, Transform::at(Vec3::new(0.1, -0.2, 0.0)))
            .expect("instance set node moves");
    }) else {
        return;
    };

    assert_eq!(retained.work, PrepareWork::RETAINED);
    retained.assert_matches_full_prepare_within_rounding("moved instance and set node");
    assert_cpu_renderer_agrees("moved instance and set node", &mut fixture, &retained.frame);
}

#[test]
fn tinting_and_hiding_around_an_instance_set_matches_a_full_prepare_on_the_retained_gpu_path() {
    type Change = fn(&mut Scene, &InstanceTargets);
    struct InstanceTargets {
        parent: NodeKey,
        node: NodeKey,
        set: InstanceSetKey,
        instance: InstanceId,
    }
    let cases: [(&str, Change, bool); 4] = [
        (
            "tinted instance set node",
            |scene, targets| {
                scene
                    .set_node_tint(targets.node, Some(Color::from_linear_rgb(0.2, 0.4, 1.0)))
                    .expect("node tints");
            },
            true,
        ),
        (
            "tinted instance",
            |scene, targets| {
                scene
                    .set_instance_tint(
                        targets.set,
                        targets.instance,
                        Some(Color::from_linear_rgb(0.1, 1.0, 0.1)),
                    )
                    .expect("instance tints");
            },
            true,
        ),
        (
            "hidden instance set node",
            |scene, targets| {
                scene.set_visible(targets.node, false).expect("node hides");
            },
            false,
        ),
        (
            "hidden instance set ancestor",
            |scene, targets| {
                scene
                    .set_visible(targets.parent, false)
                    .expect("parent hides");
            },
            false,
        ),
    ];
    for (case, change, visible) in cases {
        let mut fixture = instance_fixture();
        let targets = InstanceTargets {
            parent: fixture.parent,
            node: fixture.node,
            set: fixture.set,
            instance: fixture.first_instance,
        };
        let Some(retained) = retained_gpu_frame(&mut fixture, |scene| change(scene, &targets))
        else {
            return;
        };
        assert_eq!(retained.work, PrepareWork::RETAINED, "{case}");
        assert_eq!(
            instance_centroid(&retained.frame).is_some(),
            visible,
            "{case}: instance visibility must follow the node hierarchy"
        );
        retained.assert_matches_full_prepare(case);
    }
}

#[test]
fn mirroring_an_instance_set_node_rebakes_instead_of_reusing_its_winding() {
    let mut fixture = instance_fixture();
    let node = fixture.node;
    let Some(retained) = retained_gpu_frame(&mut fixture, |scene| {
        scene
            .set_transform(
                node,
                Transform::at(Vec3::new(0.3, 0.0, 0.0)).with_scale(Vec3::new(-1.0, 1.0, 1.0)),
            )
            .expect("instance set node mirrors");
    }) else {
        return;
    };

    assert_eq!(
        (retained.work.full_prepares, retained.work.retained_prepares),
        (1, 0),
        "a set node that starts reversing triangle winding needs the winding of a full prepare"
    );
    assert!(
        instance_centroid(&retained.frame).is_some(),
        "the mirrored instances must stay visible"
    );
    retained.assert_matches_full_prepare("mirrored instance set node");
}
