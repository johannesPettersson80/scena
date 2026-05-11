use crate::assets::Assets;
use crate::diagnostics::{Backend, Diagnostic, DiagnosticCode};
use crate::geometry::Aabb;
use crate::scene::{Camera, Quat, Scene, Transform, Vec3};

use super::super::RasterTarget;
use super::super::camera::CameraProjection;
use super::transforms::{subtract_vec3, transform_primitive};

const LARGE_SCENE_TRANSLATION_WARNING: f32 = 10_000.0;
const DEPTH_RANGE_RATIO_WARNING: f32 = 100_000.0;

pub(in crate::render) fn collect_precision_diagnostics(
    scene: &Scene,
    backend: Backend,
) -> Vec<Diagnostic> {
    let mut diagnostics = collect_camera_projection_diagnostics(scene);

    for (node, transform) in scene.node_transforms() {
        let relative_translation = subtract_vec3(transform.translation, scene.origin_shift());
        let absolute_magnitude = transform
            .translation
            .x
            .abs()
            .max(transform.translation.y.abs())
            .max(transform.translation.z.abs());
        let magnitude = relative_translation
            .x
            .abs()
            .max(relative_translation.y.abs())
            .max(relative_translation.z.abs());
        if absolute_magnitude >= LARGE_SCENE_TRANSLATION_WARNING
            && magnitude >= LARGE_SCENE_TRANSLATION_WARNING
        {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::LargeScenePrecisionRisk,
                format!(
                    "node {node:?} is {magnitude:.1} scene units from the origin; f32 transform precision may be visible"
                ),
                "use camera-relative rendering or an origin-shift policy for large-world scenes",
            ));
        }
    }

    for (node, _camera, camera) in scene.camera_nodes() {
        let (near, far) = match camera {
            Camera::Perspective(camera) => (camera.near, camera.far),
            Camera::Orthographic(camera) => (camera.near, camera.far),
        };
        if !is_valid_depth_range(camera) {
            continue;
        }
        if near > 0.0 && far.is_finite() && near.is_finite() {
            let ratio = far / near;
            if ratio > DEPTH_RANGE_RATIO_WARNING {
                diagnostics.push(Diagnostic::warning(
                    DiagnosticCode::DepthPrecisionRisk,
                    format!(
                        "camera node {node:?} has far/near ratio {ratio:.0}; depth precision may cause z-fighting"
                    ),
                    "use DepthRange::fit_sphere for focused views or tighten camera near/far planes",
                ));
            }
        }
    }

    if backend == Backend::WebGl2 {
        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::WebGl2DepthCompatibility,
            "WebGL2 disables reversed-Z depth and uses the compatibility depth profile",
            "expect reduced far/near precision; tighten camera depth ranges for WebGL2 scenes",
        ));
    }

    diagnostics
}

pub(in crate::render) fn collect_camera_projection_diagnostics(scene: &Scene) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (node, _camera_key, camera) in scene.camera_nodes() {
        if !is_valid_depth_range(camera) {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidCameraProjection,
                format!("camera node {node:?} has an invalid near/far depth range"),
                "set finite camera near and far values with far greater than near; perspective near must be positive",
            ));
        }
    }
    diagnostics
}

pub(in crate::render) fn collect_camera_visibility_diagnostics(
    scene: &Scene,
    target: RasterTarget,
) -> Vec<Diagnostic> {
    let Some(camera_key) = scene.active_camera() else {
        return Vec::new();
    };
    let Some(camera) = scene.camera(camera_key) else {
        return Vec::new();
    };
    if !is_valid_depth_range(camera) {
        return Vec::new();
    }
    let Ok(projection) = CameraProjection::from_scene(scene, camera_key, target) else {
        return Vec::new();
    };

    let origin_shift = scene.origin_shift();
    let mut vertices = 0_u32;
    let mut mesh_bound_vertices = 0_u32;
    let mut behind_vertices = 0_u32;
    let mut vertices_inside_frustum = 0_u32;

    for (renderable, transform) in scene.renderables() {
        for primitive in renderable.primitives() {
            let primitive = transform_primitive(primitive, transform, origin_shift);
            for vertex in primitive.vertices() {
                collect_visibility_sample(
                    vertex.position,
                    &projection,
                    &mut vertices,
                    &mut behind_vertices,
                    &mut vertices_inside_frustum,
                );
            }
        }
    }

    for (node, bounds) in scene.mesh_bounds_nodes() {
        let Some(transform) = scene.world_transform(node) else {
            continue;
        };
        for corner in aabb_corners(bounds) {
            mesh_bound_vertices = mesh_bound_vertices.saturating_add(1);
            collect_visibility_sample(
                transform_point(corner, transform),
                &projection,
                &mut vertices,
                &mut behind_vertices,
                &mut vertices_inside_frustum,
            );
        }
    }

    if vertices == 0 || vertices_inside_frustum > 0 {
        return Vec::new();
    }
    let source = if mesh_bound_vertices > 0 {
        "mesh bounds"
    } else {
        "direct renderable vertices"
    };
    if behind_vertices == vertices {
        return vec![Diagnostic::warning(
            DiagnosticCode::ObjectsBehindCamera,
            format!("all {source} are behind the active camera"),
            "move or frame the camera, call look_at_point, or check object placement",
        )];
    }
    vec![Diagnostic::warning(
        DiagnosticCode::SceneOutsideCameraFrustum,
        format!("{source} are outside the active camera frustum"),
        "frame the scene, adjust camera near/far and field of view, or check object placement",
    )]
}

pub(in crate::render) fn collect_asset_camera_visibility_diagnostics<F>(
    scene: &Scene,
    target: RasterTarget,
    assets: &Assets<F>,
) -> Vec<Diagnostic> {
    let Some(camera_key) = scene.active_camera() else {
        return Vec::new();
    };
    let Some(camera) = scene.camera(camera_key) else {
        return Vec::new();
    };
    if !is_valid_depth_range(camera) {
        return Vec::new();
    }
    let Ok(projection) = CameraProjection::from_scene(scene, camera_key, target) else {
        return Vec::new();
    };

    let mut vertices = 0_u32;
    let mut behind_vertices = 0_u32;
    let mut vertices_inside_frustum = 0_u32;

    for (node, mesh, _transform) in scene.mesh_nodes() {
        let Some(geometry) = assets.geometry(mesh.geometry()) else {
            continue;
        };
        let Some(transform) = scene.world_transform(node) else {
            continue;
        };
        for corner in aabb_corners(geometry.bounds()) {
            collect_visibility_sample(
                transform_point(corner, transform),
                &projection,
                &mut vertices,
                &mut behind_vertices,
                &mut vertices_inside_frustum,
            );
        }
    }

    for (node, instance_set, _transform) in scene.instance_set_nodes() {
        let Some(geometry) = assets.geometry(instance_set.geometry()) else {
            continue;
        };
        let Some(node_transform) = scene.world_transform(node) else {
            continue;
        };
        for instance in instance_set.instances() {
            let transform = compose_transform(node_transform, instance.transform());
            for corner in aabb_corners(geometry.bounds()) {
                collect_visibility_sample(
                    transform_point(corner, transform),
                    &projection,
                    &mut vertices,
                    &mut behind_vertices,
                    &mut vertices_inside_frustum,
                );
            }
        }
    }

    if vertices == 0 || vertices_inside_frustum > 0 {
        return Vec::new();
    }
    if behind_vertices == vertices {
        return vec![Diagnostic::warning(
            DiagnosticCode::ObjectsBehindCamera,
            "all asset mesh bounds are behind the active camera",
            "move or frame the camera, call look_at_point, or check object placement",
        )];
    }
    vec![Diagnostic::warning(
        DiagnosticCode::SceneOutsideCameraFrustum,
        "asset mesh bounds are outside the active camera frustum",
        "frame the scene, adjust camera near/far and field of view, or check object placement",
    )]
}

fn collect_visibility_sample(
    position: Vec3,
    projection: &CameraProjection,
    vertices: &mut u32,
    behind_vertices: &mut u32,
    vertices_inside_frustum: &mut u32,
) {
    *vertices = vertices.saturating_add(1);
    if projection
        .camera_depth(position)
        .is_some_and(|depth| depth <= 0.0)
    {
        *behind_vertices = behind_vertices.saturating_add(1);
    }
    if projection.project(position).is_some_and(|projected| {
        projected.ndc_x >= -1.0
            && projected.ndc_x <= 1.0
            && projected.ndc_y >= -1.0
            && projected.ndc_y <= 1.0
    }) {
        *vertices_inside_frustum = vertices_inside_frustum.saturating_add(1);
    }
}

fn aabb_corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}

fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    let scaled = Vec3::new(
        point.x * transform.scale.x,
        point.y * transform.scale.y,
        point.z * transform.scale.z,
    );
    add_vec3(
        rotate_vec3(transform.rotation, scaled),
        transform.translation,
    )
}

fn compose_transform(parent: Transform, child: Transform) -> Transform {
    let scaled_child_translation = Vec3::new(
        child.translation.x * parent.scale.x,
        child.translation.y * parent.scale.y,
        child.translation.z * parent.scale.z,
    );
    Transform {
        translation: add_vec3(
            parent.translation,
            rotate_vec3(parent.rotation, scaled_child_translation),
        ),
        rotation: multiply_quat(parent.rotation, child.rotation),
        scale: Vec3::new(
            parent.scale.x * child.scale.x,
            parent.scale.y * child.scale.y,
            parent.scale.z * child.scale.z,
        ),
    }
}

fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}

fn multiply_quat(left: Quat, right: Quat) -> Quat {
    normalize_quat(Quat::from_xyzw(left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y, left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x, left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w, left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z))
}

fn normalize_quat(quat: Quat) -> Quat {
    let length = (quat.x * quat.x + quat.y * quat.y + quat.z * quat.z + quat.w * quat.w).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        Quat::IDENTITY
    } else {
        Quat::from_xyzw(quat.x / length, quat.y / length, quat.z / length, quat.w / length)
    }
}

const fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn is_valid_depth_range(camera: &Camera) -> bool {
    match camera {
        Camera::Perspective(camera) => {
            camera.near.is_finite()
                && camera.far.is_finite()
                && camera.near > 0.0
                && camera.far > camera.near
        }
        Camera::Orthographic(camera) => {
            camera.near.is_finite() && camera.far.is_finite() && camera.far > camera.near
        }
    }
}
