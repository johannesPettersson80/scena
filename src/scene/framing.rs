use crate::assets::Assets;
use crate::diagnostics::LookupError;
use crate::geometry::{Aabb, GeometryDesc};
use crate::material::{Color, MaterialDesc};

use super::transforms::compose_transform;
use super::view_math::{transform_aabb, union_aabb, world_to_view};
use super::{Camera, CameraKey, DepthRange, NodeKey, NodeKind, Scene, Transform, Vec3};

mod fit;
mod grid;

use fit::{ValidFramingOptions, perspective_fit};
use grid::{GridFloorLayout, grid_geometry};

/// Options for fitting a camera to world-space bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramingOptions {
    view_direction: Vec3,
    up: Vec3,
    fill: f32,
    margin_px: f32,
    viewport_width: u32,
    viewport_height: u32,
    tighten_depth_range: bool,
}

/// Result returned by [`Scene::frame_bounds`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramingOutcome {
    /// Camera transform written into the scene.
    pub camera_transform: Transform,
    /// World-space pivot the camera was framed around.
    pub target: Vec3,
    /// Distance from `target` to the camera.
    pub distance: f32,
    /// Turntable yaw, in radians, for orbit controls that adopt this framing.
    pub yaw_radians: f32,
    /// Turntable pitch, in radians, for orbit controls that adopt this framing.
    pub pitch_radians: f32,
    /// Screen-space bounds of the framed AABB after projection.
    pub projected_rect: ScreenRect,
    /// Requested fill fraction from [`FramingOptions`].
    pub fill: f32,
    /// Requested viewport margin from [`FramingOptions`].
    pub margin_px: f32,
}

/// Pixel-space rectangle in a viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// Projected world point returned by [`Scene::project_world_point`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedPoint {
    /// Pixel x coordinate from the left edge of the viewport.
    pub x: f32,
    /// Pixel y coordinate from the top edge of the viewport.
    pub y: f32,
    /// Positive camera-space depth.
    pub depth: f32,
    /// Normalized device x coordinate.
    pub ndc_x: f32,
    /// Normalized device y coordinate.
    pub ndc_y: f32,
}

/// Options for [`Scene::add_grid_floor`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridFloorOptions {
    bounds: Option<Aabb>,
    floor_y: f32,
    padding: f32,
    line_spacing: f32,
    color: Color,
    line_color: Color,
    roughness: f32,
}

/// Node handles and world bounds for a grid floor inserted by [`Scene::add_grid_floor`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridFloorHandles {
    /// Slab mesh node.
    pub slab: NodeKey,
    /// Grid line mesh node.
    pub grid: NodeKey,
    /// World-space floor bounds.
    pub bounds: Aabb,
}

impl FramingOptions {
    /// Creates perspective framing options with conservative defaults.
    pub const fn new() -> Self {
        Self {
            view_direction: Vec3::new(0.0, 0.0, 1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            fill: 0.70,
            margin_px: 0.0,
            viewport_width: 1,
            viewport_height: 1,
            tighten_depth_range: false,
        }
    }

    /// Sets the world-space direction from the target toward the camera.
    pub const fn view_direction(mut self, view_direction: Vec3) -> Self {
        self.view_direction = view_direction;
        self
    }

    /// Alias for [`Self::view_direction`] that reads naturally at call sites.
    pub const fn look_from(self, direction: Vec3) -> Self {
        self.view_direction(direction)
    }

    /// Place the camera by azimuth and elevation in degrees.
    ///
    /// Conventions:
    /// - `azimuth_deg` is the horizontal angle from the +Z front axis.
    ///   Positive rotates toward +X (right); negative toward -X (left).
    ///   `0 deg` = front, `90 deg` = right, `+-180 deg` = back,
    ///   `-90 deg` = left.
    /// - `elevation_deg` is the vertical angle from the horizon. Positive is
    ///   above, negative is below. Values are clamped to `[-90.0, 90.0]`.
    ///
    /// Equivalent to calling [`look_from`](Self::look_from) with a unit
    /// direction derived from spherical coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// // 28 degrees to the left of front, 18 degrees above horizon.
    /// let _options = FramingOptions::new().azimuth_elevation(-28.0, 18.0);
    /// ```
    pub fn azimuth_elevation(self, azimuth_deg: f32, elevation_deg: f32) -> Self {
        let elevation_deg = elevation_deg.clamp(-90.0, 90.0);
        let az = azimuth_deg.to_radians();
        let el = elevation_deg.to_radians();
        let cos_el = el.cos();
        let look_from = Vec3::new(az.sin() * cos_el, el.sin(), az.cos() * cos_el);
        self.look_from(look_from)
    }

    /// Looks at the bounds from the positive Z direction.
    ///
    /// Places the camera in front of the target, on the +Z world axis,
    /// looking toward -Z.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().front();
    /// ```
    pub fn front(self) -> Self {
        self.azimuth_elevation(0.0, 0.0)
    }

    /// Place the camera behind the target.
    ///
    /// Places the camera on the -Z world axis, looking toward +Z.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().back();
    /// ```
    pub fn back(self) -> Self {
        self.azimuth_elevation(180.0, 0.0)
    }

    /// Place the camera to the right of the target.
    ///
    /// Places the camera on the +X world axis.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().right();
    /// ```
    pub fn right(self) -> Self {
        self.azimuth_elevation(90.0, 0.0)
    }

    /// Place the camera to the left of the target.
    ///
    /// Places the camera on the -X world axis.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().left();
    /// ```
    pub fn left(self) -> Self {
        self.azimuth_elevation(-90.0, 0.0)
    }

    /// Place the camera above the target.
    ///
    /// Places the camera on the +Y world axis, looking down.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().top();
    /// ```
    pub fn top(self) -> Self {
        self.azimuth_elevation(0.0, 90.0)
    }

    /// Place the camera below the target.
    ///
    /// Places the camera on the -Y world axis, looking up.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().bottom();
    /// ```
    pub fn bottom(self) -> Self {
        self.azimuth_elevation(0.0, -90.0)
    }

    /// Looks at the bounds from a generic isometric-style direction.
    pub const fn isometric(self) -> Self {
        self.look_from(Vec3::new(1.0, 0.65, 1.0))
    }

    /// Three-quarter view from the front-right, slightly elevated.
    ///
    /// Equivalent to `azimuth_elevation(45.0, 30.0)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().three_quarter_front_right();
    /// ```
    pub fn three_quarter_front_right(self) -> Self {
        self.azimuth_elevation(45.0, 30.0)
    }

    /// Three-quarter view from the front-left, slightly elevated.
    ///
    /// Equivalent to `azimuth_elevation(-45.0, 30.0)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().three_quarter_front_left();
    /// ```
    pub fn three_quarter_front_left(self) -> Self {
        self.azimuth_elevation(-45.0, 30.0)
    }

    /// Three-quarter view from the back-right, slightly elevated.
    ///
    /// Equivalent to `azimuth_elevation(135.0, 30.0)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().three_quarter_back_right();
    /// ```
    pub fn three_quarter_back_right(self) -> Self {
        self.azimuth_elevation(135.0, 30.0)
    }

    /// Three-quarter view from the back-left, slightly elevated.
    ///
    /// Equivalent to `azimuth_elevation(-135.0, 30.0)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::FramingOptions;
    ///
    /// let _options = FramingOptions::new().three_quarter_back_left();
    /// ```
    pub fn three_quarter_back_left(self) -> Self {
        self.azimuth_elevation(-135.0, 30.0)
    }

    /// Sets the view direction from turntable yaw and pitch radians.
    pub fn orbit(self, yaw_radians: f32, pitch_radians: f32) -> Self {
        let pitch_cos = pitch_radians.cos();
        self.look_from(Vec3::new(
            yaw_radians.sin() * pitch_cos,
            pitch_radians.sin(),
            yaw_radians.cos() * pitch_cos,
        ))
    }

    /// Sets the camera up vector used by the framing solver.
    pub const fn up(mut self, up: Vec3) -> Self {
        self.up = up;
        self
    }

    /// Sets the requested maximum viewport fill fraction.
    pub const fn fill(mut self, fill: f32) -> Self {
        self.fill = fill;
        self
    }

    /// Sets the viewport margin in pixels.
    pub const fn margin_px(mut self, margin_px: f32) -> Self {
        self.margin_px = margin_px;
        self
    }

    /// Sets the viewport size in physical pixels.
    pub const fn viewport(mut self, width: u32, height: u32) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    /// Enables near/far tightening when the caller has verified it is safe.
    pub const fn tighten_depth_range(mut self, enabled: bool) -> Self {
        self.tighten_depth_range = enabled;
        self
    }
}

impl Default for FramingOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenRect {
    /// Builds the smallest rectangle containing the projected points.
    pub fn from_points(points: &[ProjectedPoint]) -> Option<Self> {
        let first = points.first()?;
        let mut rect = Self {
            min_x: first.x,
            min_y: first.y,
            max_x: first.x,
            max_y: first.y,
        };
        for point in &points[1..] {
            rect.min_x = rect.min_x.min(point.x);
            rect.min_y = rect.min_y.min(point.y);
            rect.max_x = rect.max_x.max(point.x);
            rect.max_y = rect.max_y.max(point.y);
        }
        Some(rect)
    }

    /// Rectangle width in pixels.
    pub fn width(self) -> f32 {
        (self.max_x - self.min_x).max(0.0)
    }

    /// Rectangle height in pixels.
    pub fn height(self) -> f32 {
        (self.max_y - self.min_y).max(0.0)
    }

    /// Center x coordinate in pixels.
    pub fn center_x(self) -> f32 {
        (self.min_x + self.max_x) * 0.5
    }

    /// Center y coordinate in pixels.
    pub fn center_y(self) -> f32 {
        (self.min_y + self.max_y) * 0.5
    }

    /// Returns the larger width/height fraction of the viewport.
    pub fn fill_fraction(self, viewport_width: u32, viewport_height: u32) -> f32 {
        let width = viewport_width.max(1) as f32;
        let height = viewport_height.max(1) as f32;
        (self.width() / width).max(self.height() / height)
    }
}

impl Scene {
    /// Fits a perspective camera to world-space bounds without preparing or rendering.
    ///
    /// The camera is moved so the projected AABB fits the requested viewport
    /// fill and margin. This mutates scene camera state and marks the camera
    /// transform dirty; it does not prepare renderer resources, upload GPU
    /// data, fetch assets, or render a frame.
    ///
    /// This writes [`crate::PerspectiveCamera::aspect`] from
    /// [`FramingOptions::viewport`]. Any pre-existing aspect on the camera is
    /// ignored so the solved camera pose matches the actual target viewport.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Aabb, FramingOptions, OrbitControls, PerspectiveCamera, Scene, Transform, Vec3};
    ///
    /// let mut scene = Scene::new();
    /// let camera = scene
    ///     .add_perspective_camera(scene.root(), PerspectiveCamera::default(), Transform::default())
    ///     .unwrap();
    /// let bounds = Aabb::new(Vec3::new(-1.0, -0.5, -0.5), Vec3::new(1.0, 0.5, 0.5));
    ///
    /// let framing = scene
    ///     .frame_bounds(
    ///         camera,
    ///         bounds,
    ///         FramingOptions::new()
    ///             .isometric()
    ///             .fill(0.72)
    ///             .margin_px(48.0)
    ///             .viewport(1280, 720),
    ///     )
    ///     .unwrap();
    /// let controls = OrbitControls::from_framing(framing);
    /// # let _ = controls;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`LookupError::CameraNotFound`] when `camera` is missing,
    /// [`LookupError::UnsupportedCameraType`] for non-perspective cameras,
    /// [`LookupError::InvalidBounds`] for empty or non-projectable bounds, and
    /// [`LookupError::InvalidFramingOption`] for invalid viewport, fill, margin,
    /// or direction options.
    pub fn frame_bounds(
        &mut self,
        camera: CameraKey,
        bounds: Aabb,
        options: FramingOptions,
    ) -> Result<FramingOutcome, LookupError> {
        let options = ValidFramingOptions::new(options)?;
        let camera_node = self
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let camera_desc = self
            .cameras
            .get_mut(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;

        let Camera::Perspective(perspective) = camera_desc else {
            return Err(LookupError::UnsupportedCameraType {
                camera,
                operation: "frame_bounds",
                supported: "perspective",
            });
        };

        perspective.aspect = options.aspect();

        let fit = perspective_fit(bounds, *perspective, options)?;
        if options.tighten_depth_range {
            let depth = DepthRange::fit_sphere(fit.distance, fit.depth_radius);
            perspective.near = depth.near();
            perspective.far = depth.far();
        }

        self.align_to(camera_node, fit.camera_transform)?;
        let projected_rect = self.project_bounds_rect(
            camera,
            bounds,
            options.viewport_width,
            options.viewport_height,
        )?;

        Ok(FramingOutcome {
            camera_transform: fit.camera_transform,
            target: fit.target,
            distance: fit.distance,
            yaw_radians: fit.yaw_radians,
            pitch_radians: fit.pitch_radians,
            projected_rect,
            fill: options.fill,
            margin_px: options.margin_px,
        })
    }

    /// Projects a world-space point through a scene camera into viewport pixels.
    ///
    /// Returns `Ok(None)` when the point is outside the camera frustum or
    /// behind the camera.
    ///
    /// # Errors
    ///
    /// Returns [`LookupError::InvalidViewport`] when either viewport dimension
    /// is zero and [`LookupError::CameraNotFound`] when `camera` is missing.
    pub fn project_world_point(
        &self,
        camera: CameraKey,
        world_position: Vec3,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<Option<ProjectedPoint>, LookupError> {
        if viewport_width == 0 || viewport_height == 0 {
            return Err(LookupError::InvalidViewport {
                width: viewport_width,
                height: viewport_height,
            });
        }
        let camera_desc = self
            .camera(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let camera_node = self
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let world_from_camera = self
            .world_transform(camera_node)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let Some(view) = world_to_view(world_position, world_from_camera) else {
            return Ok(None);
        };

        let (ndc_x, ndc_y, depth) = match camera_desc {
            Camera::Perspective(camera) => {
                let depth = -view.z;
                if !depth.is_finite() || depth < camera.near || depth > camera.far {
                    return Ok(None);
                }
                let aspect = if camera.aspect.is_finite() && camera.aspect > 0.0 {
                    camera.aspect
                } else {
                    viewport_width as f32 / viewport_height as f32
                };
                let focal = (camera.vertical_fov.radians() * 0.5).tan().recip();
                (
                    view.x * focal / (aspect * depth),
                    view.y * focal / depth,
                    depth,
                )
            }
            Camera::Orthographic(camera) => {
                let depth = -view.z;
                if !depth.is_finite() || depth < camera.near || depth > camera.far {
                    return Ok(None);
                }
                let width = camera.right - camera.left;
                let height = camera.top - camera.bottom;
                if width.abs() <= f32::EPSILON || height.abs() <= f32::EPSILON {
                    return Ok(None);
                }
                (
                    (view.x - camera.left) / width * 2.0 - 1.0,
                    (view.y - camera.bottom) / height * 2.0 - 1.0,
                    depth,
                )
            }
        };

        if !ndc_x.is_finite() || !ndc_y.is_finite() {
            return Ok(None);
        }
        let x = (ndc_x * 0.5 + 0.5) * viewport_width as f32;
        let y = (1.0 - (ndc_y * 0.5 + 0.5)) * viewport_height as f32;
        Ok(Some(ProjectedPoint {
            x,
            y,
            depth,
            ndc_x,
            ndc_y,
        }))
    }

    fn project_bounds_rect(
        &self,
        camera: CameraKey,
        bounds: Aabb,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<ScreenRect, LookupError> {
        let mut points = Vec::with_capacity(8);
        for corner in aabb_corners(bounds) {
            let Some(point) =
                self.project_world_point(camera, corner, viewport_width, viewport_height)?
            else {
                return Err(LookupError::InvalidBounds {
                    reason: "framed bounds project outside the camera depth range",
                });
            };
            points.push(point);
        }
        ScreenRect::from_points(&points).ok_or(LookupError::ImportHasNoBounds)
    }

    /// Adds a matte grid floor sized from [`GridFloorOptions`].
    ///
    /// # Errors
    ///
    /// Returns [`LookupError::InvalidFramingOption`] if the floor options are
    /// invalid and [`LookupError::NodeNotFound`] if the floor mesh nodes cannot
    /// be inserted under the scene root.
    pub fn add_grid_floor<F>(
        &mut self,
        assets: &Assets<F>,
        options: GridFloorOptions,
    ) -> Result<GridFloorHandles, LookupError> {
        let layout = GridFloorLayout::new(options)?;
        let slab_geometry = assets.create_geometry(GeometryDesc::plane(layout.width, layout.depth));
        let slab_material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
            options.color,
            0.0,
            options.roughness.clamp(0.0, 1.0),
        ));
        let slab = self
            .mesh(slab_geometry, slab_material)
            .transform(Transform::at(layout.center))
            .add()?;

        let grid_geometry =
            assets.create_geometry(grid_geometry(layout.width, layout.depth, options));
        let grid_material = assets.create_material(MaterialDesc::line(options.line_color, 1.0));
        let grid = self
            .mesh(grid_geometry, grid_material)
            .transform(Transform::at(layout.center))
            .add()?;

        Ok(GridFloorHandles {
            slab,
            grid,
            bounds: layout.bounds,
        })
    }

    /// Computes union bounds for the same node evaluated at multiple transforms.
    ///
    /// This is useful for replay/animation setup where a camera or floor must
    /// contain every sampled pose, not only the current transform.
    ///
    /// # Errors
    ///
    /// Returns [`LookupError::NodeNotFound`] when `node` is missing,
    /// [`LookupError::InvalidFramingOption`] when `transforms` is empty, and
    /// [`LookupError::ImportHasNoBounds`] when the node subtree has no
    /// renderable bounds.
    pub fn bounds_for_transforms<F>(
        &self,
        node: NodeKey,
        transforms: &[Transform],
        assets: &Assets<F>,
    ) -> Result<Aabb, LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        if transforms.is_empty() {
            return Err(LookupError::InvalidFramingOption {
                field: "transforms",
                reason: "bounds_for_transforms requires at least one transform",
            });
        }
        let local_bounds = self
            .node_subtree_bounds_in_space(node, Transform::IDENTITY, assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        transforms
            .iter()
            .map(|transform| transform_aabb(local_bounds, *transform))
            .reduce(union_aabb)
            .ok_or(LookupError::ImportHasNoBounds)
    }

    fn node_subtree_bounds_in_space<F>(
        &self,
        node: NodeKey,
        space_from_node: Transform,
        assets: &Assets<F>,
    ) -> Result<Option<Aabb>, LookupError> {
        let node_ref = self
            .nodes
            .get(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        let mut bounds = match &node_ref.kind {
            NodeKind::Mesh(mesh) => {
                let geometry =
                    assets
                        .geometry(mesh.geometry())
                        .ok_or(LookupError::GeometryNotFound {
                            node,
                            geometry: mesh.geometry(),
                        })?;
                Some(transform_aabb(geometry.bounds(), space_from_node))
            }
            NodeKind::InstanceSet(instance_set) => {
                let instance_set = self
                    .instance_sets
                    .get(*instance_set)
                    .ok_or(LookupError::InstanceSetNotFound(*instance_set))?;
                let geometry = assets.geometry(instance_set.geometry()).ok_or(
                    LookupError::GeometryNotFound {
                        node,
                        geometry: instance_set.geometry(),
                    },
                )?;
                instance_set
                    .instances()
                    .map(|instance| {
                        transform_aabb(
                            geometry.bounds(),
                            compose_transform(space_from_node, instance.transform()),
                        )
                    })
                    .reduce(union_aabb)
            }
            _ => self
                .node_bounds
                .get(&node)
                .map(|bounds| transform_aabb(*bounds, space_from_node)),
        };

        for child in &node_ref.children {
            let child_ref = self
                .nodes
                .get(*child)
                .ok_or(LookupError::NodeNotFound(*child))?;
            let child_space = compose_transform(space_from_node, child_ref.transform);
            if let Some(child_bounds) =
                self.node_subtree_bounds_in_space(*child, child_space, assets)?
            {
                bounds =
                    Some(bounds.map_or(child_bounds, |bounds| union_aabb(bounds, child_bounds)));
            }
        }
        Ok(bounds)
    }
}

fn validate_bounds(bounds: Aabb) -> Result<(), LookupError> {
    if !bounds.min.is_finite() || !bounds.max.is_finite() {
        return Err(LookupError::InvalidBounds {
            reason: "bounds must be finite",
        });
    }
    let extent = bounds.max - bounds.min;
    if extent.x < 0.0 || extent.y < 0.0 || extent.z < 0.0 {
        return Err(LookupError::InvalidBounds {
            reason: "bounds min must be less than or equal to max",
        });
    }
    if extent.length_squared() <= f32::EPSILON {
        return Err(LookupError::ImportHasNoBounds);
    }
    Ok(())
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
