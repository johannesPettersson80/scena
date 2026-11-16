//! The stage the template-reuse cases play on, and the changes they make to it.

use crate::assets::{Assets, EnvironmentDesc, EnvironmentHandle, GeometryHandle, MaterialHandle};
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::scene::{CameraKey, DirectionalLight, NodeKey, Scene, Transform, Vec3};

use super::support::{
    Backend, RetainedFrame, orbit_camera, retained_frame_after_configured, visible_centroid,
};

pub(super) const SIZE: u32 = 64;
pub(super) const RADIUS: f32 = 2.4;
pub(super) const BACKENDS: [Backend; 2] = [Backend::Cpu, Backend::Gpu];
pub(super) const RED: Color = Color::from_linear_rgb(1.0, 0.15, 0.1);

/// A camera on the orbit circle, two lights, a box at the origin and a smaller
/// box beside it.
pub(super) struct Stage {
    pub(super) scene: Scene,
    pub(super) assets: Assets,
    pub(super) camera: CameraKey,
    pub(super) subject: NodeKey,
    pub(super) neighbour: NodeKey,
    pub(super) box_geometry: GeometryHandle,
    pub(super) material: MaterialHandle,
    /// Environment every renderer of the stage is lit with, if any.
    pub(super) environment: Option<EnvironmentHandle>,
}

impl Stage {
    pub(super) fn new(lit: bool) -> Self {
        let assets = Assets::new();
        let material = |color: Color| {
            assets.create_material(if lit {
                MaterialDesc::pbr_metallic_roughness(color, 0.0, 0.6)
            } else {
                MaterialDesc::unlit(color)
            })
        };
        let box_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.6, 0.6));
        let small = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
        let blue = material(Color::from_linear_rgb(0.2, 0.5, 0.9));
        let green = material(Color::from_linear_rgb(0.3, 0.8, 0.3));

        let mut scene = Scene::new();
        let camera = scene.add_default_camera().expect("camera inserts");
        orbit_camera(&mut scene, camera, 20.0, RADIUS);
        // Lit from two opposite sides, so no orbit position looks only at
        // faces that are in the dark.
        for yaw in [30.0, 210.0] {
            scene
                .directional_light(DirectionalLight::default())
                .transform(Transform::IDENTITY.rotate_x_deg(-35.0).rotate_y_deg(yaw))
                .add()
                .expect("light inserts");
        }
        let subject = scene
            .mesh(box_geometry, blue)
            .add()
            .expect("subject inserts");
        let neighbour = scene
            .mesh(small, green)
            .transform(Transform::at(Vec3::new(0.7, 0.1, 0.2)))
            .add()
            .expect("neighbour inserts");
        Self {
            scene,
            assets,
            camera,
            subject,
            neighbour,
            box_geometry,
            material: blue,
            environment: None,
        }
    }

    /// The stage a backend's retained path is exercised with.
    ///
    /// The GPU shades lit materials in the shader, so lit geometry is what its
    /// retained path draws. The CPU renderer bakes lit shading per vertex from
    /// the camera; that is a case of its own, and every other CPU case
    /// uses unlit materials so that only the thing under test decides.
    pub(super) fn for_backend(backend: Backend) -> Self {
        Self::new(backend == Backend::Gpu)
    }

    pub(super) fn after(
        &mut self,
        backend: Backend,
        change: impl FnOnce(&mut Scene),
    ) -> Option<RetainedFrame> {
        let environment = self.environment;
        let frame = retained_frame_after_configured(
            backend,
            SIZE,
            &mut self.scene,
            &self.assets,
            |renderer| {
                if let Some(environment) = environment {
                    renderer.set_environment(environment);
                }
            },
            change,
        )?;
        assert!(
            visible_centroid(&frame.reference, SIZE).is_some(),
            "{backend:?}: a case that draws nothing proves nothing"
        );
        Some(frame)
    }

    /// [`Self::after`] once an earlier case of the same test has shown that
    /// the backend is available.
    pub(super) fn then(
        &mut self,
        backend: Backend,
        change: impl FnOnce(&mut Scene),
    ) -> RetainedFrame {
        self.after(backend, change)
            .expect("the backend built a renderer for the first case of this test")
    }
}

pub(super) fn orbit_to(camera: CameraKey, yaw_degrees: f32) -> impl FnOnce(&mut Scene) {
    move |scene| orbit_camera(scene, camera, yaw_degrees, RADIUS)
}

pub(super) fn move_to(node: NodeKey, position: Vec3) -> impl FnOnce(&mut Scene) {
    move |scene| {
        scene
            .set_transform(node, Transform::at(position))
            .expect("node moves");
    }
}

pub(super) fn tint(node: NodeKey) -> impl FnOnce(&mut Scene) {
    move |scene| {
        scene
            .set_node_tint(node, Some(RED))
            .expect("node takes a tint");
    }
}

pub(super) fn set_visible(node: NodeKey, visible: bool) -> impl FnOnce(&mut Scene) {
    move |scene| {
        scene
            .set_visible(node, visible)
            .expect("node visibility changes");
    }
}

pub(super) fn solid_environment(
    assets: &Assets,
    name: &str,
    radiance: [f32; 3],
) -> EnvironmentHandle {
    assets.create_environment(
        EnvironmentDesc::from_cubemap_radiance(
            format!("scena://generated/movement-tests/{name}"),
            16,
            std::array::from_fn(|_| vec![radiance; 16 * 16]),
        )
        .expect("solid cubemap is valid"),
    )
}
