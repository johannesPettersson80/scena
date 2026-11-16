//! Ray construction, bounds tests, triangle/BVH tests, and typed hit results.

use crate::Assets;
use crate::diagnostics::LookupError;
use crate::geometry::{GeometryDesc, GeometryTopology, Primitive, TriangleBvh};
use crate::material::MaterialKind;
use crate::scene::{Camera, CameraKey, InstanceId, NodeKey, Scene, Vec3, WorldTransform};

mod bounds_hit;
mod geometry_hit;
mod math;
use bounds_hit::pick_recorded_bounds;
use geometry_hit::{GeometryInstanceHitInput, hit_geometry, hit_geometry_instance, hit_triangle};

use math::{
    add_vec3, cross, normalize, normalize_optional, ray_hits_bounds, ray_triangle_intersection,
    rotate_vec3, scale_vec3, subtract_vec3, transform_point, triangle_bounds,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorPosition {
    x: f32,
    y: f32,
    units: CursorUnits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorUnits {
    Logical,
    Physical,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub physical_width: u32,
    pub physical_height: u32,
    pub device_pixel_ratio: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    Node(NodeKey),
    Instance { node: NodeKey, instance: InstanceId },
}

/// The nearest hit of a pick, in the current rendered scene pose.
///
/// `distance` is measured in world-space units along the normalized camera ray,
/// and `world_position` is `ray_origin + ray_direction * distance`.
///
/// A triangle-accurate hit (any hit from
/// [`Scene::pick_with_assets`](crate::Scene::pick_with_assets), and a hit on a
/// renderable primitive from any pick) reports `normal`, the normalized
/// geometric normal of the triangle as it is rendered. The node's exact world
/// matrix is applied before intersection, shear included, and the winding of a
/// mirrored (negative-determinant) transform is restored, so the
/// normal keeps pointing out of the front face. Triangles collapsed by a
/// singular transform are not hittable.
///
/// A bounds-accurate hit (a mesh or instance-set node reached by a pick that
/// takes no `Assets`, such as [`Scene::pick`](crate::Scene::pick)) lies on the
/// node's recorded bounds rather than on its surface and has no `normal`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// The scene node or concrete instance that was hit.
    pub target: HitTarget,
    /// World-space distance from the camera-ray origin to the intersection.
    pub distance: f32,
    /// World-space intersection position.
    pub world_position: Vec3,
    /// Normalized geometric normal of the rendered front face. `None` for a
    /// degenerate face and for a bounds-accurate hit.
    pub normal: Option<Vec3>,
}

/// Deterministic work counters from one asset-aware picking query.
///
/// This is an inspection surface for performance evidence. The ordinary
/// picking methods do not collect counters; call
/// [`Scene::pick_with_assets_profiled`](crate::Scene::pick_with_assets_profiled)
/// only when diagnostics or benchmark evidence needs them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PickingMetrics {
    pub mesh_nodes_considered: u64,
    pub instance_sets_considered: u64,
    pub mesh_bounds_tests: u64,
    pub mesh_bounds_rejections: u64,
    pub bvh_node_bounds_tests: u64,
    pub static_bvh_cache_hits: u64,
    pub static_bvh_cache_misses: u64,
    pub deformed_bvh_builds: u64,
    pub triangles_considered: u64,
    pub triangle_bounds_tests: u64,
    pub ray_triangle_intersection_tests: u64,
    pub deformed_vertices_materialized: u64,
    pub deformed_vertex_bytes_materialized: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InteractionContext {
    hover: Option<HitTarget>,
    primary_selection: Option<HitTarget>,
    revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl CursorPosition {
    pub const fn logical(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            units: CursorUnits::Logical,
        }
    }

    pub const fn physical(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            units: CursorUnits::Physical,
        }
    }

    fn physical_xy(self, viewport: Viewport) -> (f32, f32) {
        match self.units {
            CursorUnits::Logical => (
                self.x * viewport.device_pixel_ratio,
                self.y * viewport.device_pixel_ratio,
            ),
            CursorUnits::Physical => (self.x, self.y),
        }
    }
}

impl Viewport {
    pub fn new(physical_width: u32, physical_height: u32, device_pixel_ratio: f32) -> Option<Self> {
        (physical_width > 0 && physical_height > 0 && device_pixel_ratio.is_finite()).then_some(
            Self {
                physical_width,
                physical_height,
                device_pixel_ratio: device_pixel_ratio.max(0.001),
            },
        )
    }
}

impl Hit {
    pub const fn target(&self) -> HitTarget {
        self.target
    }
}

impl InteractionContext {
    pub const fn hover(&self) -> Option<HitTarget> {
        self.hover
    }

    pub fn set_hover(&mut self, hover: Option<HitTarget>) {
        if self.hover != hover {
            self.hover = hover;
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub const fn primary_selection(&self) -> Option<HitTarget> {
        self.primary_selection
    }

    pub fn set_primary_selection(&mut self, primary_selection: Option<HitTarget>) {
        if self.primary_selection != primary_selection {
            self.primary_selection = primary_selection;
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }
}

pub(crate) fn pick_scene(
    scene: &Scene,
    camera: CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
) -> Result<Option<Hit>, LookupError> {
    let ray = camera_ray(scene, camera, cursor, viewport)?;
    let mut metrics = PickingMetrics::default();

    let renderables = pick_renderables::<false>(scene, ray, &mut metrics);
    Ok(nearest_hit(renderables, pick_recorded_bounds(scene, ray)?))
}

pub(crate) fn pick_scene_with_assets<F>(
    scene: &Scene,
    assets: &Assets<F>,
    camera: CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
) -> Result<Option<Hit>, LookupError> {
    let mut metrics = PickingMetrics::default();
    pick_scene_with_assets_impl::<false, F>(scene, assets, camera, cursor, viewport, &mut metrics)
}

pub(crate) fn pick_scene_with_assets_profiled<F>(
    scene: &Scene,
    assets: &Assets<F>,
    camera: CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
) -> Result<(Option<Hit>, PickingMetrics), LookupError> {
    let mut metrics = PickingMetrics::default();
    let hit = pick_scene_with_assets_impl::<true, F>(
        scene,
        assets,
        camera,
        cursor,
        viewport,
        &mut metrics,
    )?;
    Ok((hit, metrics))
}

fn pick_scene_with_assets_impl<const PROFILE: bool, F>(
    scene: &Scene,
    assets: &Assets<F>,
    camera: CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
    metrics: &mut PickingMetrics,
) -> Result<Option<Hit>, LookupError> {
    let ray = camera_ray(scene, camera, cursor, viewport)?;
    raycast_scene_with_assets_impl::<PROFILE, F>(scene, assets, ray, metrics)
}

fn raycast_scene_with_assets_impl<const PROFILE: bool, F>(
    scene: &Scene,
    assets: &Assets<F>,
    ray: Ray,
    metrics: &mut PickingMetrics,
) -> Result<Option<Hit>, LookupError> {
    let mut best = pick_renderables::<PROFILE>(scene, ray, metrics);

    for (node, mesh, transform) in scene.mesh_nodes() {
        if PROFILE {
            metrics.mesh_nodes_considered = metrics.mesh_nodes_considered.saturating_add(1);
        }
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(LookupError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        let Some(material) = assets.material(mesh.material()) else {
            continue;
        };
        if is_stroke_material(material.kind()) {
            continue;
        }
        let skin_matrices = scene.skin_matrices(node);
        let vertices = geometry
            .deformed_vertices(scene.morph_weights(node), skin_matrices.as_deref())
            .map_err(|_| invalid_skin_binding(&geometry, skin_matrices.as_deref()))?;
        let deformed = matches!(&vertices, std::borrow::Cow::Owned(_));
        if PROFILE && deformed {
            let vertex_count = vertices.len() as u64;
            metrics.deformed_vertices_materialized = metrics
                .deformed_vertices_materialized
                .saturating_add(vertex_count);
            metrics.deformed_vertex_bytes_materialized =
                metrics.deformed_vertex_bytes_materialized.saturating_add(
                    vertex_count.saturating_mul(
                        std::mem::size_of::<crate::geometry::GeometryVertex>() as u64,
                    ),
                );
        }
        if let Some(hit) = hit_geometry::<PROFILE>(
            node, &geometry, &vertices, deformed, transform, ray, metrics,
        ) {
            best = nearest_hit(best, Some(hit));
        }
    }
    for (node, instance_set, node_transform) in scene.instance_set_nodes() {
        if PROFILE {
            metrics.instance_sets_considered = metrics.instance_sets_considered.saturating_add(1);
        }
        let geometry =
            assets
                .geometry(instance_set.geometry())
                .ok_or(LookupError::GeometryNotFound {
                    node,
                    geometry: instance_set.geometry(),
                })?;
        let Some(material) = assets.material(instance_set.material()) else {
            continue;
        };
        if is_stroke_material(material.kind()) {
            continue;
        }
        let vertices = geometry
            .deformed_vertices(None, None)
            .map_err(|_| invalid_skin_binding(&geometry, None))?;
        for instance in instance_set
            .instances()
            .filter(|instance| instance.visible())
        {
            if let Some(hit) = hit_geometry_instance::<PROFILE>(
                GeometryInstanceHitInput {
                    node,
                    instance: instance.id(),
                    geometry: &geometry,
                    vertices: &vertices,
                    node_transform,
                    instance_transform: instance_set.placement(instance),
                    ray,
                },
                metrics,
            ) {
                best = nearest_hit(best, Some(hit));
            }
        }
    }

    Ok(best)
}

const fn is_stroke_material(kind: MaterialKind) -> bool {
    matches!(
        kind,
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge
    )
}

fn pick_renderables<const PROFILE: bool>(
    scene: &Scene,
    ray: Ray,
    metrics: &mut PickingMetrics,
) -> Option<Hit> {
    let mut best = None;
    for (node, renderable, transform) in scene.pickable_renderables() {
        for primitive in renderable.primitives() {
            best = nearest_hit(
                best,
                hit_primitive::<PROFILE>(node, primitive, transform, ray, metrics),
            );
        }
    }
    best
}

fn camera_ray(
    scene: &Scene,
    camera: CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
) -> Result<Ray, LookupError> {
    let camera_desc = scene
        .camera(camera)
        .ok_or(LookupError::CameraNotFound(camera))?;
    let camera_node = scene
        .camera_node(camera)
        .ok_or(LookupError::CameraNotFound(camera))?;
    // The ray is the camera-local ray carried through the camera's exact world
    // matrix, the inverse of the view matrix both backends render with.
    let world_from_camera = scene
        .resolved_world(camera_node)
        .ok_or(LookupError::CameraNotFound(camera))?;
    let (x, y) = cursor.physical_xy(viewport);
    let ndc_x = x / viewport.physical_width as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - y / viewport.physical_height as f32 * 2.0;
    match camera_desc {
        Camera::Perspective(camera) => {
            let aspect = if camera.aspect.is_finite() && camera.aspect > 0.0 {
                camera.aspect
            } else {
                viewport.physical_width.max(1) as f32 / viewport.physical_height.max(1) as f32
            };
            let half_fov = camera.vertical_fov.radians() * 0.5;
            let tan_half_fov = half_fov.tan();
            let local_direction = normalize(Vec3::new(
                ndc_x * aspect * tan_half_fov,
                ndc_y * tan_half_fov,
                -1.0,
            ));
            Ok(Ray {
                origin: world_from_camera.translation(),
                direction: normalize(world_from_camera.transform_vector(local_direction)),
            })
        }
        Camera::Orthographic(camera) => {
            let width = camera.right - camera.left;
            let height = camera.top - camera.bottom;
            let local_origin = Vec3::new(
                camera.left + (ndc_x + 1.0) * 0.5 * width,
                camera.bottom + (ndc_y + 1.0) * 0.5 * height,
                0.0,
            );
            Ok(Ray {
                origin: transform_point(local_origin, world_from_camera),
                direction: normalize(world_from_camera.transform_vector(Vec3::new(0.0, 0.0, -1.0))),
            })
        }
    }
}

fn hit_primitive<const PROFILE: bool>(
    node: NodeKey,
    primitive: &Primitive,
    transform: WorldTransform,
    ray: Ray,
    metrics: &mut PickingMetrics,
) -> Option<Hit> {
    if PROFILE {
        metrics.triangles_considered = metrics.triangles_considered.saturating_add(1);
    }
    let [a, b, c] = primitive.vertices();
    let a = transform_point(a.position, transform);
    let b = transform_point(b.position, transform);
    let c = transform_point(c.position, transform);
    hit_triangle::<PROFILE>(
        HitTarget::Node(node),
        [a, b, c],
        transform.is_mirrored(),
        ray,
        metrics,
    )
}

fn nearest_hit(left: Option<Hit>, right: Option<Hit>) -> Option<Hit> {
    match (left, right) {
        (Some(left), Some(right)) if right.distance < left.distance => Some(right),
        (Some(left), Some(_)) => Some(left),
        (None, Some(right)) => Some(right),
        (Some(left), None) => Some(left),
        (None, None) => None,
    }
}

fn invalid_skin_binding(
    geometry: &GeometryDesc,
    matrices: Option<&[crate::geometry::SkinningMatrix]>,
) -> LookupError {
    let joint_count = geometry
        .skin()
        .and_then(|skin| skin.joints().iter().flatten().max().copied())
        .map_or(0, |maximum| maximum.saturating_add(1));
    LookupError::InvalidSkinBinding {
        joint_count,
        inverse_bind_count: matrices.map_or(0, |matrices| matrices.len()),
    }
}

#[cfg(test)]
mod tests;
