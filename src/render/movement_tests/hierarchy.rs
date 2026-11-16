//! Node hierarchy, tint and visibility on the retained prepare path.
//!
//! Every retained draw has to follow the whole chain of ancestors above its
//! node, the tint of its node, and visibility anywhere in that chain, exactly
//! as a full prepare resolves them.

use crate::assets::Assets;
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::scene::{DirectionalLight, NodeKey, Scene, Transform, Vec3};

use super::support::{
    Backend, PrepareWork, RetainedFrame, fresh_full_prepare_frame, prepare_and_render,
    retained_frame_after,
};

const SIZE: u32 = 64;

struct HierarchyFixture {
    scene: Scene,
    assets: Assets,
    grandparent: NodeKey,
    parent: NodeKey,
    mesh: NodeKey,
    line: NodeKey,
}

/// A lit box and a line under `grandparent -> parent`, plus a witness mesh
/// outside the hierarchy.
fn hierarchy_fixture() -> HierarchyFixture {
    let assets = Assets::new();
    let box_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
    let box_material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_linear_rgb(0.8, 0.8, 0.3),
        0.0,
        0.8,
    ));
    let line_geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(0.0, -0.3, 0.0),
        Vec3::new(0.0, 0.3, 0.0),
    ));
    let line_material = assets.create_material(MaterialDesc::line(Color::WHITE, 3.0));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    scene
        .mesh(box_geometry, box_material)
        .transform(Transform::at(Vec3::new(0.0, -0.8, 0.0)).with_uniform_scale(0.4))
        .add()
        .expect("witness mesh inserts");
    let grandparent = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("grandparent inserts");
    let parent = scene
        .add_empty(grandparent, Transform::at(Vec3::new(-0.3, 0.0, 0.0)))
        .expect("parent inserts");
    let mesh = scene
        .mesh(box_geometry, box_material)
        .parent(parent)
        .transform(Transform::at(Vec3::new(-0.2, 0.0, 0.0)).rotate_z_deg(15.0))
        .add()
        .expect("mesh inserts");
    let line = scene
        .mesh(line_geometry, line_material)
        .parent(parent)
        .transform(Transform::at(Vec3::new(0.25, 0.0, 0.0)))
        .add()
        .expect("line inserts");
    HierarchyFixture {
        scene,
        assets,
        grandparent,
        parent,
        mesh,
        line,
    }
}

#[test]
fn retained_meshes_and_strokes_follow_ancestors_tint_and_visibility_like_a_full_prepare() {
    type Change = fn(&mut Scene, &HierarchyFixtureKeys);
    struct HierarchyFixtureKeys {
        grandparent: NodeKey,
        parent: NodeKey,
        mesh: NodeKey,
        line: NodeKey,
    }
    // The flag says whether the case moves a draw; those may differ from a
    // full prepare by float rounding, the rest must match it byte for byte.
    let cases: [(&str, bool, Change); 6] = [
        ("moved grandparent", true, |scene, keys| {
            scene
                .set_transform(keys.grandparent, Transform::at(Vec3::new(0.5, 0.3, 0.0)))
                .expect("grandparent moves");
        }),
        ("sheared ancestors", true, |scene, keys| {
            // Non-uniform grandparent scale over a rotated parent: the mesh
            // and line world matrices are sheared.
            scene
                .set_transform(
                    keys.grandparent,
                    Transform::at(Vec3::new(0.3, 0.1, 0.0)).with_scale(Vec3::new(1.6, 0.7, 1.0)),
                )
                .expect("grandparent scales");
            scene
                .set_transform(
                    keys.parent,
                    Transform::at(Vec3::new(-0.2, 0.1, 0.0)).rotate_z_deg(40.0),
                )
                .expect("parent rotates");
        }),
        ("tinted mesh and line", false, |scene, keys| {
            scene
                .set_node_tint(keys.mesh, Some(Color::from_linear_rgb(0.2, 0.4, 1.0)))
                .expect("mesh tints");
            scene
                .set_node_tint(keys.line, Some(Color::from_linear_rgb(1.0, 0.2, 0.2)))
                .expect("line tints");
        }),
        ("hidden mesh", false, |scene, keys| {
            scene.set_visible(keys.mesh, false).expect("mesh hides");
        }),
        ("hidden parent", false, |scene, keys| {
            scene.set_visible(keys.parent, false).expect("parent hides");
        }),
        ("hidden grandparent after a move", false, |scene, keys| {
            scene
                .set_transform(keys.parent, Transform::at(Vec3::new(0.4, 0.2, 0.0)))
                .expect("parent moves");
            scene
                .set_visible(keys.grandparent, false)
                .expect("grandparent hides");
        }),
    ];
    for (case, moves_a_draw, change) in cases {
        let mut fixture = hierarchy_fixture();
        let keys = HierarchyFixtureKeys {
            grandparent: fixture.grandparent,
            parent: fixture.parent,
            mesh: fixture.mesh,
            line: fixture.line,
        };
        let Some(retained) = retained_frame_after(
            Backend::Gpu,
            SIZE,
            &mut fixture.scene,
            &fixture.assets,
            |scene| change(scene, &keys),
        ) else {
            return;
        };
        assert_eq!(retained.work, PrepareWork::RETAINED, "{case}");
        if moves_a_draw {
            retained.assert_matches_full_prepare_within_rounding(case);
        } else {
            retained.assert_matches_full_prepare(case);
        }
    }
}

#[test]
fn re_showing_a_moved_subtree_draws_it_where_it_is_now() {
    let mut fixture = hierarchy_fixture();
    let (grandparent, parent) = (fixture.grandparent, fixture.parent);
    let Some(mut renderer) = Backend::Gpu.renderer(SIZE, SIZE) else {
        return;
    };
    let camera = fixture.scene.active_camera().expect("active camera");
    let mut frame = |scene: &mut Scene, assets: &Assets| {
        prepare_and_render(&mut renderer, scene, assets, camera)
    };
    frame(&mut fixture.scene, &fixture.assets);

    fixture
        .scene
        .set_visible(grandparent, false)
        .expect("grandparent hides");
    frame(&mut fixture.scene, &fixture.assets);
    fixture
        .scene
        .set_transform(parent, Transform::at(Vec3::new(0.45, 0.35, 0.0)))
        .expect("parent moves while hidden");
    frame(&mut fixture.scene, &fixture.assets);
    fixture
        .scene
        .set_visible(grandparent, true)
        .expect("grandparent shows");
    let (shown, work) = frame(&mut fixture.scene, &fixture.assets);

    assert_eq!(work, PrepareWork::RETAINED);
    let reference = fresh_full_prepare_frame(
        Backend::Gpu,
        SIZE,
        SIZE,
        &mut fixture.scene,
        &fixture.assets,
        camera,
    );
    RetainedFrame {
        frame: shown,
        work,
        reference,
    }
    .assert_matches_full_prepare_within_rounding("subtree moved while hidden, then shown");
}

#[test]
fn instanced_strokes_rebake_instead_of_collapsing_onto_their_node() {
    // A line-material instance set is baked per instance, so each stroke's
    // world matrix is `node x instance`. Re-deriving it from the node alone
    // would collapse every instance onto the node origin, so the retained
    // path has to hand such a scene back to a full prepare.
    let assets = Assets::new();
    let line_geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(0.0, -0.25, 0.0),
        Vec3::new(0.0, 0.25, 0.0),
    ));
    let line_material = assets.create_material(MaterialDesc::line(Color::WHITE, 3.0));
    let box_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let box_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let mover = scene
        .mesh(box_geometry, box_material)
        .transform(Transform::at(Vec3::new(0.0, -0.7, 0.0)))
        .add()
        .expect("mover inserts");
    let (_node, set) = scene
        .add_instance_set_node(
            scene.root(),
            line_geometry,
            line_material,
            Transform::IDENTITY,
        )
        .expect("line instance set inserts");
    for x in [-0.6, 0.0, 0.6] {
        scene
            .push_instance(set, Transform::at(Vec3::new(x, 0.2, 0.0)))
            .expect("instance inserts");
    }

    let Some(retained) = retained_frame_after(Backend::Gpu, SIZE, &mut scene, &assets, |scene| {
        scene
            .set_transform(mover, Transform::at(Vec3::new(0.3, -0.7, 0.0)))
            .expect("unrelated mesh moves");
    }) else {
        return;
    };

    assert_eq!(
        (retained.work.full_prepares, retained.work.retained_prepares),
        (1, 0),
        "strokes baked per instance cannot be re-placed from their node"
    );
    retained.assert_matches_full_prepare("instanced strokes after an unrelated move");
}
