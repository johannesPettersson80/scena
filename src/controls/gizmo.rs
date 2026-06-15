use crate::diagnostics::LookupError;
use crate::{
    Assets, Color, GeometryDesc, Hit, HitTarget, MaterialDesc, NodeKey, Scene, Transform, Vec3,
};

const EPSILON: f32 = 1.0e-5;
const DEFAULT_SIZE: f32 = 1.0;
const DEFAULT_LINE_WIDTH_PX: f32 = 2.0;

/// Direct-manipulation mode for [`TransformGizmo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

/// Axis used by transform gizmo constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GizmoAxis {
    X,
    Y,
    Z,
}

/// Coordinate space used when resolving gizmo axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GizmoSpace {
    World,
    Local,
    ViewAligned,
}

/// Drag constraint for a [`TransformGizmo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GizmoConstraint {
    Axis(GizmoAxis),
    Plane(GizmoAxis),
    ViewPlane,
}

/// Pointer ray used by platform-neutral gizmo math.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoRay {
    origin: Vec3,
    direction: Vec3,
}

/// Helper geometry inserted for a transform gizmo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformGizmoHelpers {
    nodes: Vec<NodeKey>,
}

/// Platform-neutral transform manipulator.
///
/// The gizmo consumes pointer rays supplied by the host's camera/picking layer and returns a
/// new [`Transform`] or, with the `scene-host` feature, a `scena.visual_patch.v1` update. It
/// never owns selection, undo/redo, snapping, collision, or an application document model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformGizmo {
    mode: GizmoMode,
    space: GizmoSpace,
    constraint: Option<GizmoConstraint>,
    size: f32,
    line_width_px: f32,
}

impl TransformGizmo {
    pub const fn new(mode: GizmoMode) -> Self {
        Self {
            mode,
            space: GizmoSpace::World,
            constraint: None,
            size: DEFAULT_SIZE,
            line_width_px: DEFAULT_LINE_WIDTH_PX,
        }
    }

    pub const fn mode(self) -> GizmoMode {
        self.mode
    }

    pub const fn space(self) -> GizmoSpace {
        self.space
    }

    pub const fn constraint(self) -> Option<GizmoConstraint> {
        self.constraint
    }

    pub const fn size(self) -> f32 {
        self.size
    }

    pub const fn line_width_px(self) -> f32 {
        self.line_width_px
    }

    pub const fn with_space(mut self, space: GizmoSpace) -> Self {
        self.space = space;
        self
    }

    pub const fn with_constraint(mut self, constraint: GizmoConstraint) -> Self {
        self.constraint = Some(constraint);
        self
    }

    pub const fn without_constraint(mut self) -> Self {
        self.constraint = None;
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > EPSILON {
            self.size = size;
        }
        self
    }

    pub fn with_line_width_px(mut self, width_px: f32) -> Self {
        if width_px.is_finite() && width_px > 0.0 {
            self.line_width_px = width_px;
        }
        self
    }

    /// Extracts the node target from an existing picking hit.
    ///
    /// Instance hits return the instance-set node; hosts that need per-instance document
    /// semantics should map the hit to their own selection model before emitting a patch.
    pub const fn target_from_hit(hit: Hit) -> NodeKey {
        match hit.target() {
            HitTarget::Node(node) | HitTarget::Instance { node, .. } => node,
        }
    }

    /// Computes the transformed value for a drag from `start_ray` to `current_ray`.
    pub fn drag_transform(
        self,
        start: Transform,
        start_ray: GizmoRay,
        current_ray: GizmoRay,
    ) -> Option<Transform> {
        match self.mode {
            GizmoMode::Translate => self.drag_translate(start, start_ray, current_ray),
            GizmoMode::Rotate => self.drag_rotate(start, start_ray, current_ray),
            GizmoMode::Scale => self.drag_scale(start, start_ray, current_ray),
        }
    }

    /// Inserts local-axis line helpers under `target` and marks them as helper-on-top.
    ///
    /// The helpers use line materials, which are visible helper strokes but are ignored by
    /// picking in v1.7. This keeps normal scene picks focused on the manipulated target.
    pub fn add_helpers<F>(
        self,
        scene: &mut Scene,
        assets: &Assets<F>,
        target: NodeKey,
    ) -> Result<TransformGizmoHelpers, LookupError> {
        let axes = [
            (GizmoAxis::X, Color::RED),
            (GizmoAxis::Y, Color::GREEN),
            (GizmoAxis::Z, Color::BLUE),
        ];
        let mut nodes = Vec::with_capacity(axes.len());
        for (axis, color) in axes {
            let geometry =
                assets.create_geometry(GeometryDesc::line(Vec3::ZERO, axis.vector() * self.size));
            let material = assets.create_material(MaterialDesc::line(color, self.line_width_px));
            let node = scene
                .mesh(geometry, material)
                .parent(target)
                .transform(Transform::IDENTITY)
                .add()?;
            scene.set_helper_on_top(node, true)?;
            nodes.push(node);
        }
        Ok(TransformGizmoHelpers { nodes })
    }

    #[cfg(feature = "scene-host")]
    pub fn to_visual_patch(self, target_handle: u64, transform: Transform) -> crate::VisualPatchV1 {
        let mut patch = crate::VisualPatchV1::default();
        patch.transforms.push(crate::VisualPatchTransformV1 {
            node: target_handle,
            transform,
        });
        patch
    }

    fn drag_translate(
        self,
        start: Transform,
        start_ray: GizmoRay,
        current_ray: GizmoRay,
    ) -> Option<Transform> {
        let delta = match self.effective_constraint_for_translate() {
            GizmoConstraint::Axis(axis) => {
                let axis = self.axis_for(axis, start)?;
                let start_t = ray_axis_parameter(start.translation, axis, start_ray)?;
                let current_t = ray_axis_parameter(start.translation, axis, current_ray)?;
                axis * (current_t - start_t)
            }
            GizmoConstraint::Plane(axis) => {
                let normal = self.axis_for(axis, start)?;
                let start_point = ray_plane_intersection(start.translation, normal, start_ray)?;
                let current_point = ray_plane_intersection(start.translation, normal, current_ray)?;
                current_point - start_point
            }
            GizmoConstraint::ViewPlane => {
                let normal = view_plane_normal(start_ray)?;
                let start_point = ray_plane_intersection(start.translation, normal, start_ray)?;
                let current_point = ray_plane_intersection(start.translation, normal, current_ray)?;
                current_point - start_point
            }
        };
        let mut transformed = start;
        transformed.translation += finite_vec3(delta)?;
        Some(transformed)
    }

    fn drag_rotate(
        self,
        start: Transform,
        start_ray: GizmoRay,
        current_ray: GizmoRay,
    ) -> Option<Transform> {
        let axis = match self
            .constraint
            .unwrap_or(GizmoConstraint::Axis(GizmoAxis::Y))
        {
            GizmoConstraint::Axis(axis) | GizmoConstraint::Plane(axis) => {
                self.axis_for(axis, start)?
            }
            GizmoConstraint::ViewPlane => view_plane_normal(start_ray)?,
        };
        let start_point = ray_plane_intersection(start.translation, axis, start_ray)?;
        let current_point = ray_plane_intersection(start.translation, axis, current_ray)?;
        let start_vec = normalize(start_point - start.translation)?;
        let current_vec = normalize(current_point - start.translation)?;
        let angle = axis
            .dot(start_vec.cross(current_vec))
            .atan2(start_vec.dot(current_vec));
        if !angle.is_finite() {
            return None;
        }
        let delta = glam::Quat::from_axis_angle(axis, angle);
        let mut transformed = start;
        let rotation = delta * start.rotation;
        transformed.rotation = if rotation.is_finite() && rotation.length_squared() > EPSILON {
            rotation.normalize()
        } else {
            start.rotation
        };
        Some(transformed)
    }

    fn drag_scale(
        self,
        start: Transform,
        start_ray: GizmoRay,
        current_ray: GizmoRay,
    ) -> Option<Transform> {
        let mut transformed = start;
        match self.constraint.unwrap_or(GizmoConstraint::ViewPlane) {
            GizmoConstraint::Axis(axis) => {
                let world_axis = self.axis_for(axis, start)?;
                let start_t = ray_axis_parameter(start.translation, world_axis, start_ray)?;
                if start_t.abs() <= EPSILON {
                    return None;
                }
                let current_t = ray_axis_parameter(start.translation, world_axis, current_ray)?;
                let factor = finite_scale_factor(current_t / start_t)?;
                transformed.scale = apply_axis_scale(start.scale, axis, factor)?;
            }
            GizmoConstraint::Plane(axis) => {
                let normal = self.axis_for(axis, start)?;
                let start_point = ray_plane_intersection(start.translation, normal, start_ray)?;
                let current_point = ray_plane_intersection(start.translation, normal, current_ray)?;
                let factor = distance_factor(start.translation, start_point, current_point)?;
                transformed.scale = apply_plane_scale(start.scale, axis, factor)?;
            }
            GizmoConstraint::ViewPlane => {
                let normal = view_plane_normal(start_ray)?;
                let start_point = ray_plane_intersection(start.translation, normal, start_ray)?;
                let current_point = ray_plane_intersection(start.translation, normal, current_ray)?;
                let factor = distance_factor(start.translation, start_point, current_point)?;
                transformed.scale = start.scale * factor;
            }
        }
        finite_vec3(transformed.scale)?;
        Some(transformed)
    }

    fn axis_for(self, axis: GizmoAxis, start: Transform) -> Option<Vec3> {
        match self.space {
            GizmoSpace::World | GizmoSpace::ViewAligned => Some(axis.vector()),
            GizmoSpace::Local => normalize(start.rotation * axis.vector()),
        }
    }

    fn effective_constraint_for_translate(self) -> GizmoConstraint {
        self.constraint.unwrap_or(match self.space {
            GizmoSpace::ViewAligned => GizmoConstraint::ViewPlane,
            GizmoSpace::World | GizmoSpace::Local => GizmoConstraint::Plane(GizmoAxis::Z),
        })
    }
}

impl GizmoRay {
    pub fn new(origin: Vec3, direction: Vec3) -> Option<Self> {
        if !origin.is_finite() {
            return None;
        }
        Some(Self {
            origin,
            direction: normalize(direction)?,
        })
    }

    pub const fn origin(self) -> Vec3 {
        self.origin
    }

    pub const fn direction(self) -> Vec3 {
        self.direction
    }
}

impl TransformGizmoHelpers {
    pub fn nodes(&self) -> &[NodeKey] {
        &self.nodes
    }
}

impl GizmoAxis {
    const fn vector(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }
}

fn ray_axis_parameter(pivot: Vec3, axis: Vec3, ray: GizmoRay) -> Option<f32> {
    let axis = normalize(axis)?;
    let ray_dir = ray.direction;
    let offset = pivot - ray.origin;
    let b = axis.dot(ray_dir);
    let c = axis.dot(offset);
    let f = ray_dir.dot(offset);
    let denom = 1.0 - b * b;
    if denom.abs() <= EPSILON || !denom.is_finite() {
        return None;
    }
    finite_f32((b * f - c) / denom)
}

fn ray_plane_intersection(pivot: Vec3, normal: Vec3, ray: GizmoRay) -> Option<Vec3> {
    let normal = normalize(normal)?;
    let denom = normal.dot(ray.direction);
    if denom.abs() <= EPSILON || !denom.is_finite() {
        return None;
    }
    let t = finite_f32(normal.dot(pivot - ray.origin) / denom)?;
    finite_vec3(ray.origin + ray.direction * t)
}

fn view_plane_normal(ray: GizmoRay) -> Option<Vec3> {
    normalize(ray.direction)
}

fn normalize(vector: Vec3) -> Option<Vec3> {
    let length_sq = vector.length_squared();
    if !length_sq.is_finite() || length_sq <= EPSILON * EPSILON {
        return None;
    }
    Some(vector / length_sq.sqrt())
}

fn finite_vec3(vector: Vec3) -> Option<Vec3> {
    vector.is_finite().then_some(vector)
}

fn finite_f32(value: f32) -> Option<f32> {
    value.is_finite().then_some(value)
}

fn finite_scale_factor(value: f32) -> Option<f32> {
    finite_f32(value).map(|factor| factor.max(EPSILON))
}

fn distance_factor(pivot: Vec3, start_point: Vec3, current_point: Vec3) -> Option<f32> {
    let start_distance = (start_point - pivot).length();
    if !start_distance.is_finite() || start_distance <= EPSILON {
        return None;
    }
    finite_scale_factor((current_point - pivot).length() / start_distance)
}

fn apply_axis_scale(scale: Vec3, axis: GizmoAxis, factor: f32) -> Option<Vec3> {
    let mut result = scale;
    match axis {
        GizmoAxis::X => result.x *= factor,
        GizmoAxis::Y => result.y *= factor,
        GizmoAxis::Z => result.z *= factor,
    }
    finite_vec3(result)
}

fn apply_plane_scale(scale: Vec3, normal_axis: GizmoAxis, factor: f32) -> Option<Vec3> {
    let mut result = scale;
    match normal_axis {
        GizmoAxis::X => {
            result.y *= factor;
            result.z *= factor;
        }
        GizmoAxis::Y => {
            result.x *= factor;
            result.z *= factor;
        }
        GizmoAxis::Z => {
            result.x *= factor;
            result.y *= factor;
        }
    }
    finite_vec3(result)
}
