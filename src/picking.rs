//! Ray construction, bounds tests, triangle/BVH tests, and typed hit results.

use crate::Assets;
use crate::diagnostics::LookupError;
use crate::geometry::{GeometryDesc, GeometryTopology, Primitive};
use crate::material::{Color, MaterialKind};
use crate::scene::{Camera, CameraKey, NodeKey, Scene, Transform, Vec3};

mod math;

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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub target: HitTarget,
    pub distance: f32,
    pub world_position: Vec3,
    pub normal: Option<Vec3>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractionStyle {
    color: Color,
    outline_width_px: f32,
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

impl InteractionStyle {
    pub const fn outline(color: Color, outline_width_px: f32) -> Self {
        Self {
            color,
            outline_width_px: positive_or(outline_width_px, 2.0),
        }
    }

    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn outline_width_px(self) -> f32 {
        self.outline_width_px
    }
}

impl Default for InteractionStyle {
    fn default() -> Self {
        Self::outline(Color::WHITE, 2.0)
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

const fn positive_or(value: f32, fallback: f32) -> f32 {
    if !value.is_finite() || value <= 0.0 {
        fallback
    } else {
        value
    }
}

pub(crate) fn pick_scene(
    scene: &Scene,
    camera: CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
) -> Result<Option<Hit>, LookupError> {
    let ray = camera_ray(scene, camera, cursor, viewport)?;

    Ok(pick_renderables(scene, ray))
}

pub(crate) fn pick_scene_with_assets<F>(
    scene: &Scene,
    assets: &Assets<F>,
    camera: CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
) -> Result<Option<Hit>, LookupError> {
    let ray = camera_ray(scene, camera, cursor, viewport)?;
    let mut best = pick_renderables(scene, ray);

    for (node, mesh, _local_transform) in scene.mesh_nodes() {
        let transform = scene
            .world_transform(node)
            .ok_or(LookupError::NodeNotFound(node))?;
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
        if let Some(hit) = hit_geometry(node, &geometry, transform, ray) {
            best = nearest_hit(best, Some(hit));
        }
    }
    for (node, instance_set, _local_transform) in scene.instance_set_nodes() {
        let node_transform = scene
            .world_transform(node)
            .ok_or(LookupError::NodeNotFound(node))?;
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
        for instance in instance_set.instances() {
            if let Some(hit) =
                hit_geometry_instance(node, &geometry, node_transform, instance.transform(), ray)
            {
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

fn pick_renderables(scene: &Scene, ray: Ray) -> Option<Hit> {
    scene
        .pickable_renderables()
        .filter_map(|(node, renderable, transform)| {
            renderable
                .primitives()
                .iter()
                .filter_map(|primitive| hit_primitive(node, primitive, transform, ray))
                .min_by(|left, right| left.distance.total_cmp(&right.distance))
        })
        .min_by(|left, right| left.distance.total_cmp(&right.distance))
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
    let world_from_camera = scene
        .world_transform(camera_node)
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
                origin: world_from_camera.translation,
                direction: normalize(rotate_vec3(world_from_camera.rotation, local_direction)),
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
                direction: normalize(rotate_vec3(
                    world_from_camera.rotation,
                    Vec3::new(0.0, 0.0, -1.0),
                )),
            })
        }
    }
}

fn hit_primitive(
    node: NodeKey,
    primitive: &Primitive,
    transform: Transform,
    ray: Ray,
) -> Option<Hit> {
    let [a, b, c] = primitive.vertices();
    let a = transform_point(a.position, transform);
    let b = transform_point(b.position, transform);
    let c = transform_point(c.position, transform);
    let (min, max) = triangle_bounds(a, b, c);
    if !ray_hits_bounds(ray, min, max) {
        return None;
    }
    let (distance, _u, _v) = ray_triangle_intersection(ray, a, b, c)?;
    Some(Hit {
        target: HitTarget::Node(node),
        distance,
        world_position: add_vec3(ray.origin, scale_vec3(ray.direction, distance)),
        normal: normalize_optional(cross(subtract_vec3(b, a), subtract_vec3(c, a))),
    })
}

fn hit_geometry(
    node: NodeKey,
    geometry: &GeometryDesc,
    transform: Transform,
    ray: Ray,
) -> Option<Hit> {
    if geometry.topology() != GeometryTopology::Triangles {
        return None;
    }
    geometry
        .indices()
        .chunks_exact(3)
        .filter_map(|indices| {
            let a = geometry.vertices().get(indices[0] as usize)?;
            let b = geometry.vertices().get(indices[1] as usize)?;
            let c = geometry.vertices().get(indices[2] as usize)?;
            hit_triangle(
                node,
                transform_point(a.position, transform),
                transform_point(b.position, transform),
                transform_point(c.position, transform),
                ray,
            )
        })
        .min_by(|left, right| left.distance.total_cmp(&right.distance))
}

fn hit_geometry_instance(
    node: NodeKey,
    geometry: &GeometryDesc,
    node_transform: Transform,
    instance_transform: Transform,
    ray: Ray,
) -> Option<Hit> {
    if geometry.topology() != GeometryTopology::Triangles {
        return None;
    }
    geometry
        .indices()
        .chunks_exact(3)
        .filter_map(|indices| {
            let a = geometry.vertices().get(indices[0] as usize)?;
            let b = geometry.vertices().get(indices[1] as usize)?;
            let c = geometry.vertices().get(indices[2] as usize)?;
            hit_triangle(
                node,
                transform_point(
                    transform_point(a.position, instance_transform),
                    node_transform,
                ),
                transform_point(
                    transform_point(b.position, instance_transform),
                    node_transform,
                ),
                transform_point(
                    transform_point(c.position, instance_transform),
                    node_transform,
                ),
                ray,
            )
        })
        .min_by(|left, right| left.distance.total_cmp(&right.distance))
}

fn hit_triangle(node: NodeKey, a: Vec3, b: Vec3, c: Vec3, ray: Ray) -> Option<Hit> {
    let (min, max) = triangle_bounds(a, b, c);
    if !ray_hits_bounds(ray, min, max) {
        return None;
    }
    let (distance, _u, _v) = ray_triangle_intersection(ray, a, b, c)?;
    Some(Hit {
        target: HitTarget::Node(node),
        distance,
        world_position: add_vec3(ray.origin, scale_vec3(ray.direction, distance)),
        normal: normalize_optional(cross(subtract_vec3(b, a), subtract_vec3(c, a))),
    })
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

#[cfg(test)]
mod tests;
