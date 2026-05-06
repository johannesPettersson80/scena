//! Ray construction, bounds tests, triangle/BVH tests, and typed hit results.

use crate::diagnostics::LookupError;
use crate::geometry::Primitive;
use crate::material::Color;
use crate::scene::{CameraKey, NodeKey, Quat, Scene, Transform, Vec3};

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
        self.hover = hover;
    }

    pub const fn primary_selection(&self) -> Option<HitTarget> {
        self.primary_selection
    }

    pub fn set_primary_selection(&mut self, primary_selection: Option<HitTarget>) {
        self.primary_selection = primary_selection;
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
    if scene.camera(camera).is_none() {
        return Err(LookupError::CameraNotFound(camera));
    }
    let (x, y) = cursor.physical_xy(viewport);
    let point = Vec3::new(
        x / viewport.physical_width as f32 * 2.0 - 1.0,
        1.0 - y / viewport.physical_height as f32 * 2.0,
        0.0,
    );

    Ok(scene
        .pickable_renderables()
        .filter_map(|(node, renderable, transform)| {
            renderable
                .primitives()
                .iter()
                .filter_map(|primitive| hit_primitive(node, primitive, transform, point))
                .min_by(|left, right| left.distance.total_cmp(&right.distance))
        })
        .min_by(|left, right| left.distance.total_cmp(&right.distance)))
}

fn hit_primitive(
    node: NodeKey,
    primitive: &Primitive,
    transform: Transform,
    point: Vec3,
) -> Option<Hit> {
    let [a, b, c] = primitive.vertices();
    let a = transform_point(a.position, transform);
    let b = transform_point(b.position, transform);
    let c = transform_point(c.position, transform);
    let (u, v, w) = barycentric_xy(point, a, b, c)?;
    if u < 0.0 || v < 0.0 || w < 0.0 {
        return None;
    }
    let z = a.z * u + b.z * v + c.z * w;
    Some(Hit {
        target: HitTarget::Node(node),
        distance: z.abs(),
        world_position: Vec3::new(point.x, point.y, z),
        normal: None,
    })
}

fn barycentric_xy(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, f32, f32)> {
    let v0 = Vec3::new(b.x - a.x, b.y - a.y, 0.0);
    let v1 = Vec3::new(c.x - a.x, c.y - a.y, 0.0);
    let v2 = Vec3::new(point.x - a.x, point.y - a.y, 0.0);
    let d00 = dot_xy(v0, v0);
    let d01 = dot_xy(v0, v1);
    let d11 = dot_xy(v1, v1);
    let d20 = dot_xy(v2, v0);
    let d21 = dot_xy(v2, v1);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() <= f32::EPSILON {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    Some((u, v, w))
}

fn dot_xy(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y
}

fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    let scaled = Vec3::new(
        point.x * transform.scale.x,
        point.y * transform.scale.y,
        point.z * transform.scale.z,
    );
    let rotated = rotate_vec3(transform.rotation, scaled);
    Vec3::new(
        rotated.x + transform.translation.x,
        rotated.y + transform.translation.y,
        rotated.z + transform.translation.z,
    )
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
