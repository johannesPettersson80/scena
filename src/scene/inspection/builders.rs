use crate::assets::{Assets, GeometryHandle, MaterialHandle};
use crate::geometry::GeometryTopology;

use super::super::{Camera, CameraKey, InstanceId, NodeKey, Scene, Vec3, WorldTransform};
use super::{
    SceneCameraFrustumInspection, SceneDrawInspection, SceneMaterialInspection,
    SceneNormalInspection,
};

impl Scene {
    pub(super) fn inspect_draw_list<F>(
        &self,
        assets: Option<&Assets<F>>,
    ) -> Vec<SceneDrawInspection> {
        let Some(assets) = assets else {
            return Vec::new();
        };
        let mut draw_list = Vec::new();
        for (node, mesh, node_world) in self.mesh_nodes() {
            append_draw_entry(
                &mut draw_list,
                assets,
                DrawEntryInput {
                    node,
                    instance: None,
                    geometry: mesh.geometry(),
                    material: mesh.material(),
                    world: node_world,
                },
            );
        }
        for (node, instance_set, node_world) in self.instance_set_nodes() {
            for instance in instance_set
                .instances()
                .filter(|instance| instance.visible())
            {
                append_draw_entry(
                    &mut draw_list,
                    assets,
                    DrawEntryInput {
                        node,
                        instance: Some(instance.id()),
                        geometry: instance_set.geometry(),
                        material: instance_set.material(),
                        world: node_world.child(instance.transform()),
                    },
                );
            }
        }
        draw_list
    }

    pub(super) fn inspect_normal_overlays<F>(
        &self,
        assets: Option<&Assets<F>>,
    ) -> Vec<SceneNormalInspection> {
        let Some(assets) = assets else {
            return Vec::new();
        };
        let mut overlays = Vec::new();
        for (node, mesh, node_world) in self.mesh_nodes() {
            append_normal_overlay(
                &mut overlays,
                assets,
                NormalOverlayInput {
                    node,
                    instance: None,
                    geometry: mesh.geometry(),
                    world: node_world,
                },
            );
        }
        for (node, instance_set, node_world) in self.instance_set_nodes() {
            for instance in instance_set
                .instances()
                .filter(|instance| instance.visible())
            {
                append_normal_overlay(
                    &mut overlays,
                    assets,
                    NormalOverlayInput {
                        node,
                        instance: Some(instance.id()),
                        geometry: instance_set.geometry(),
                        world: node_world.child(instance.transform()),
                    },
                );
            }
        }
        overlays
    }

    pub(super) fn inspect_camera_frustums(&self) -> Vec<SceneCameraFrustumInspection> {
        self.cameras
            .iter()
            .filter_map(|(camera, desc)| {
                let node = self.camera_node(camera)?;
                let world = self.resolved_world(node)?;
                Some(camera_frustum(camera, node, desc.clone(), world))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct NormalOverlayInput {
    node: NodeKey,
    instance: Option<InstanceId>,
    geometry: GeometryHandle,
    world: WorldTransform,
}

const NORMAL_DEBUG_LENGTH: f32 = 0.1;

fn append_normal_overlay<F>(
    overlays: &mut Vec<SceneNormalInspection>,
    assets: &Assets<F>,
    input: NormalOverlayInput,
) {
    let Some(geometry) = assets.geometry(input.geometry) else {
        return;
    };
    let segments = geometry
        .vertices()
        .iter()
        .map(|vertex| {
            let start = input.world.transform_point(vertex.position);
            let end = input.world.transform_point(Vec3::new(
                vertex.position.x + vertex.normal.x * NORMAL_DEBUG_LENGTH,
                vertex.position.y + vertex.normal.y * NORMAL_DEBUG_LENGTH,
                vertex.position.z + vertex.normal.z * NORMAL_DEBUG_LENGTH,
            ));
            [start, end]
        })
        .collect();
    overlays.push(SceneNormalInspection {
        node: input.node,
        instance: input.instance,
        geometry: input.geometry,
        length: NORMAL_DEBUG_LENGTH,
        segments,
    });
}

#[derive(Debug, Clone, Copy)]
struct DrawEntryInput {
    node: NodeKey,
    instance: Option<InstanceId>,
    geometry: GeometryHandle,
    material: MaterialHandle,
    world: WorldTransform,
}

fn append_draw_entry<F>(
    draw_list: &mut Vec<SceneDrawInspection>,
    assets: &Assets<F>,
    input: DrawEntryInput,
) {
    let Some(geometry) = assets.geometry(input.geometry) else {
        return;
    };
    let material_preview = assets
        .material(input.material)
        .map(|material| SceneMaterialInspection::new(input.material, material, assets));
    draw_list.push(SceneDrawInspection {
        node: input.node,
        instance: input.instance,
        geometry: input.geometry,
        material: input.material,
        material_preview,
        topology: geometry.topology(),
        primitive_count: primitive_count(geometry.topology(), geometry.indices().len()),
        vertex_count: geometry.vertices().len(),
        index_count: geometry.indices().len(),
        local_bounds: geometry.bounds(),
        world: input.world,
    });
}

const fn primitive_count(topology: GeometryTopology, index_count: usize) -> usize {
    match topology {
        GeometryTopology::Triangles => index_count / 3,
        GeometryTopology::Lines => index_count / 2,
    }
}

fn camera_frustum(
    camera: CameraKey,
    node: NodeKey,
    desc: Camera,
    world: WorldTransform,
) -> SceneCameraFrustumInspection {
    let (near, far, local_corners) = match desc {
        Camera::Perspective(camera_desc) => {
            let near = camera_desc.near;
            let far = camera_desc.far;
            let aspect = positive_or(camera_desc.aspect, 1.0);
            let half_fov_tan = (camera_desc.vertical_fov.radians() * 0.5).tan();
            let near_half_y = near * half_fov_tan;
            let near_half_x = near_half_y * aspect;
            let far_half_y = far * half_fov_tan;
            let far_half_x = far_half_y * aspect;
            (
                near,
                far,
                frustum_corners_from_extents(
                    -near_half_x,
                    near_half_x,
                    -near_half_y,
                    near_half_y,
                    -near,
                    -far_half_x,
                    far_half_x,
                    -far_half_y,
                    far_half_y,
                    -far,
                ),
            )
        }
        Camera::Orthographic(camera_desc) => (
            camera_desc.near,
            camera_desc.far,
            frustum_corners_from_extents(
                camera_desc.left,
                camera_desc.right,
                camera_desc.bottom,
                camera_desc.top,
                -camera_desc.near,
                camera_desc.left,
                camera_desc.right,
                camera_desc.bottom,
                camera_desc.top,
                -camera_desc.far,
            ),
        ),
    };
    SceneCameraFrustumInspection {
        camera,
        node,
        near,
        far,
        corners: local_corners.map(|corner| world.transform_point(corner)),
    }
}

#[allow(clippy::too_many_arguments)]
const fn frustum_corners_from_extents(
    near_left: f32,
    near_right: f32,
    near_bottom: f32,
    near_top: f32,
    near_z: f32,
    far_left: f32,
    far_right: f32,
    far_bottom: f32,
    far_top: f32,
    far_z: f32,
) -> [Vec3; 8] {
    [
        Vec3::new(near_left, near_bottom, near_z),
        Vec3::new(near_right, near_bottom, near_z),
        Vec3::new(near_right, near_top, near_z),
        Vec3::new(near_left, near_top, near_z),
        Vec3::new(far_left, far_bottom, far_z),
        Vec3::new(far_right, far_bottom, far_z),
        Vec3::new(far_right, far_top, far_z),
        Vec3::new(far_left, far_top, far_z),
    ]
}

const fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}
