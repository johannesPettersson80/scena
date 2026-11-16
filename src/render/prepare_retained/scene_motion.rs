//! What moved in a scene since it was prepared: the camera, or its content.
//!
//! The scene bumps one transform revision for any node that moves. The camera
//! is a node too, so orbiting bumps the same counter as animating a mesh, yet
//! the two ask very different things of a prepared template: a camera move
//! only changes what is culled and how blended triangles are ordered, while a
//! moving mesh, light or joint can invalidate anything baked from its
//! placement. The retained prepare path tells the two apart by comparing what
//! it captured here at the last full prepare with the scene as it is now.

use crate::scene::{CameraKey, NodeKind, Scene, Transform};

use super::prepare::transforms::world_from_model_matrix;

/// The camera a prepare culled and ordered for.
///
/// Covers everything that defines the view: which camera is active (the scene
/// does not bump a revision when that changes), its projection, and where its
/// node is in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render) struct PreparedView {
    camera: Option<CameraKey>,
    camera_revision: u64,
    world_from_camera: Option<[u32; 16]>,
}

impl PreparedView {
    pub(in crate::render) fn of(scene: &Scene) -> Self {
        let camera = scene.active_camera();
        let world_from_camera = camera
            .and_then(|camera| scene.camera_node(camera))
            .and_then(|node| scene.resolved_world(node))
            .map(|world| world_from_model_matrix(world, scene.origin_shift()).map(f32::to_bits));
        Self {
            camera,
            camera_revision: scene.camera_revision(),
            world_from_camera,
        }
    }

    /// The active camera. Which nodes are visible follows from its layer
    /// mask, so another camera can show another set of nodes.
    pub(in crate::render) const fn camera(&self) -> Option<CameraKey> {
        self.camera
    }
}

/// Where the content of a scene is: the local transform of every node except
/// cameras that carry nothing, and the local placement of every instance.
///
/// World placement follows from these and from the hierarchy, and a hierarchy
/// change bumps the scene's structure revision, which invalidates a template
/// before placement is ever compared. Equal local transforms under an equal
/// structure revision therefore mean every world matrix the bake read is
/// bit-for-bit what it was.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::render) struct ContentPlacement {
    bits: Vec<u32>,
}

impl ContentPlacement {
    pub(in crate::render) fn of(scene: &Scene) -> Self {
        let mut bits = Vec::new();
        visit_content_placements(scene, |placement| {
            bits.extend_from_slice(placement);
            true
        });
        Self { bits }
    }

    /// Whether the scene's content is exactly where this placement recorded
    /// it. Compares in place, without building a second placement.
    pub(in crate::render) fn matches(&self, scene: &Scene) -> bool {
        let mut remaining = self.bits.as_slice();
        let same_prefix = visit_content_placements(scene, |placement| {
            match remaining.split_at_checked(placement.len()) {
                Some((recorded, rest)) if recorded == placement => {
                    remaining = rest;
                    true
                }
                _ => false,
            }
        });
        same_prefix && remaining.is_empty()
    }
}

/// Calls `visit` with the bits of the local placement of every content node
/// and instance, in scene order, until it returns `false`. Returns whether it
/// never did.
fn visit_content_placements(scene: &Scene, mut visit: impl FnMut(&[u32]) -> bool) -> bool {
    for (key, transform) in scene.node_transforms() {
        let Some(node) = scene.node(key) else {
            continue;
        };
        match node.kind() {
            // A camera with nothing parented to it is the view, not content.
            NodeKind::Camera(_) if node.children().is_empty() => continue,
            NodeKind::InstanceSet(set) => {
                if let Some(set) = scene.instance_set(*set) {
                    // An instance placed by an exact matrix can move while
                    // the `Transform` that matrix decomposes to stays put.
                    let exact = set.has_exact_placements();
                    for instance in set.instances() {
                        if !visit(&transform_bits(instance.transform())) {
                            return false;
                        }
                        if exact
                            && !visit(
                                &set.placement(instance)
                                    .matrix()
                                    .to_cols_array()
                                    .map(f32::to_bits),
                            )
                        {
                            return false;
                        }
                    }
                }
            }
            _ => {}
        }
        if !visit(&transform_bits(transform)) {
            return false;
        }
    }
    true
}

fn transform_bits(transform: Transform) -> [u32; 10] {
    [
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
        transform.rotation.x,
        transform.rotation.y,
        transform.rotation.z,
        transform.rotation.w,
        transform.scale.x,
        transform.scale.y,
        transform.scale.z,
    ]
    .map(f32::to_bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::geometry::GeometryDesc;
    use crate::material::{Color, MaterialDesc};
    use crate::scene::{DirectionalLight, OrthographicCamera, PerspectiveCamera, Vec3};

    struct Fixture {
        scene: Scene,
        camera: CameraKey,
        mesh: crate::scene::NodeKey,
        light: crate::scene::NodeKey,
        empty: crate::scene::NodeKey,
        set: crate::scene::InstanceSetKey,
        instance: crate::scene::InstanceId,
    }

    fn fixture() -> Fixture {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        let camera = scene.add_default_camera().expect("camera inserts");
        let empty = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("empty inserts");
        let mesh = scene
            .mesh(geometry, material)
            .parent(empty)
            .add()
            .expect("mesh inserts");
        let light = scene
            .directional_light(DirectionalLight::default())
            .add()
            .expect("light inserts");
        let (_node, set) = scene
            .add_instance_set_node(scene.root(), geometry, material, Transform::IDENTITY)
            .expect("instance set inserts");
        let instance = scene
            .push_instance(set, Transform::IDENTITY)
            .expect("instance inserts");
        Fixture {
            scene,
            camera,
            mesh,
            light,
            empty,
            set,
            instance,
        }
    }

    fn moved() -> Transform {
        Transform::at(Vec3::new(0.25, 0.5, -1.0)).rotate_y_deg(30.0)
    }

    #[test]
    fn moving_the_camera_changes_the_view_and_leaves_the_content_in_place() {
        let mut fixture = fixture();
        let view = PreparedView::of(&fixture.scene);
        let content = ContentPlacement::of(&fixture.scene);

        let camera_node = fixture
            .scene
            .camera_node(fixture.camera)
            .expect("camera node");
        fixture
            .scene
            .set_transform(camera_node, moved())
            .expect("camera moves");

        assert_ne!(PreparedView::of(&fixture.scene), view);
        assert!(
            content.matches(&fixture.scene),
            "a camera move must not read as moved content"
        );
        assert_eq!(ContentPlacement::of(&fixture.scene), content);
    }

    #[test]
    fn anything_but_a_bare_camera_moving_is_moved_content() {
        type Move = fn(&mut Fixture);
        let cases: [(&str, Move); 4] = [
            ("mesh", |fixture| {
                fixture
                    .scene
                    .set_transform(fixture.mesh, moved())
                    .expect("mesh moves");
            }),
            ("ancestor of a mesh", |fixture| {
                fixture
                    .scene
                    .set_transform(fixture.empty, moved())
                    .expect("empty moves");
            }),
            ("light", |fixture| {
                fixture
                    .scene
                    .set_transform(fixture.light, moved())
                    .expect("light moves");
            }),
            ("instance", |fixture| {
                fixture
                    .scene
                    .set_instance_transform(fixture.set, fixture.instance, moved())
                    .expect("instance moves");
            }),
        ];
        for (case, change) in cases {
            let mut fixture = fixture();
            let content = ContentPlacement::of(&fixture.scene);
            let view = PreparedView::of(&fixture.scene);

            change(&mut fixture);

            assert_eq!(
                PreparedView::of(&fixture.scene),
                view,
                "{case}: content moving is not a view change"
            );
            assert!(
                !content.matches(&fixture.scene),
                "{case}: moving it must read as moved content"
            );
        }
    }

    #[test]
    fn a_camera_that_carries_something_moves_content_with_it() {
        let mut fixture = fixture();
        let camera_node = fixture
            .scene
            .camera_node(fixture.camera)
            .expect("camera node");
        fixture
            .scene
            .directional_light(DirectionalLight::default())
            .parent(camera_node)
            .add()
            .expect("headlight inserts");
        let content = ContentPlacement::of(&fixture.scene);

        fixture
            .scene
            .set_transform(camera_node, moved())
            .expect("camera moves with its headlight");

        assert!(
            !content.matches(&fixture.scene),
            "a headlight parented to the camera moves when the camera does"
        );
    }

    #[test]
    fn the_view_follows_the_active_camera_and_its_projection() {
        let mut fixture = fixture();
        let second = fixture
            .scene
            .add_perspective_camera(
                fixture.scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, 2.0)),
            )
            .expect("second camera inserts");
        let with_second_camera = PreparedView::of(&fixture.scene);
        fixture
            .scene
            .set_active_camera(second)
            .expect("second camera activates");
        assert_ne!(
            PreparedView::of(&fixture.scene),
            with_second_camera,
            "switching the active camera is a view change even at the same placement"
        );

        let before_projection = PreparedView::of(&fixture.scene);
        fixture
            .scene
            .set_camera(
                second,
                crate::scene::Camera::Orthographic(OrthographicCamera::default()),
            )
            .expect("projection changes");
        assert_ne!(PreparedView::of(&fixture.scene), before_projection);

        assert_ne!(PreparedView::of(&Scene::new()), before_projection);
    }
}
