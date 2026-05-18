use crate::diagnostics::LookupError;
use crate::geometry::{Aabb, GeometryDesc, GeometryTopology, GeometryVertex};

use super::{GridFloorOptions, validate_bounds};
use crate::material::Color;
use crate::scene::Vec3;

#[derive(Debug, Clone, Copy)]
pub(super) struct GridFloorLayout {
    pub(super) center: Vec3,
    pub(super) width: f32,
    pub(super) depth: f32,
    pub(super) bounds: Aabb,
}

impl GridFloorOptions {
    /// Creates a matte, dark, bounds-sized floor configuration.
    pub fn new() -> Self {
        Self {
            bounds: None,
            floor_y: 0.0,
            padding: 0.4,
            line_spacing: 0.24,
            color: Color::from_srgb_u8(54, 59, 69),
            line_color: Color::from_srgb_u8(69, 75, 87),
            roughness: 0.96,
        }
    }

    /// Sizes and centers the floor from world-space bounds.
    pub const fn under_bounds(mut self, bounds: Aabb) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Sets the world-space Y plane for the floor.
    pub const fn floor_y(mut self, floor_y: f32) -> Self {
        self.floor_y = floor_y;
        self
    }

    /// Sets extra padding around the supplied bounds.
    pub const fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Sets grid line spacing in world units.
    pub const fn line_spacing(mut self, line_spacing: f32) -> Self {
        self.line_spacing = line_spacing;
        self
    }

    /// Sets the matte slab color.
    pub const fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets the grid line color.
    pub const fn line_color(mut self, line_color: Color) -> Self {
        self.line_color = line_color;
        self
    }

    /// Sets slab roughness. Values are clamped during material creation.
    pub const fn roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }
}

impl Default for GridFloorOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl GridFloorLayout {
    pub(super) fn new(options: GridFloorOptions) -> Result<Self, LookupError> {
        if !options.floor_y.is_finite()
            || !options.padding.is_finite()
            || options.padding < 0.0
            || !options.line_spacing.is_finite()
            || options.line_spacing <= 0.0
        {
            return Err(LookupError::InvalidFramingOption {
                field: "grid_floor",
                reason: "grid floor options must be finite and non-negative",
            });
        }
        let (center_x, center_z, width, depth) = if let Some(bounds) = options.bounds {
            validate_bounds(bounds)?;
            let center = bounds.center();
            let extent = bounds.max - bounds.min;
            (
                center.x,
                center.z,
                (extent.x + options.padding * 2.0).max(options.line_spacing),
                (extent.z + options.padding * 2.0).max(options.line_spacing),
            )
        } else {
            (0.0, 0.0, 1.0, 1.0)
        };
        let half_width = width * 0.5;
        let half_depth = depth * 0.5;
        let bounds = Aabb::new(
            Vec3::new(
                center_x - half_width,
                options.floor_y,
                center_z - half_depth,
            ),
            Vec3::new(
                center_x + half_width,
                options.floor_y,
                center_z + half_depth,
            ),
        );
        Ok(Self {
            center: Vec3::new(center_x, options.floor_y, center_z),
            width,
            depth,
            bounds,
        })
    }
}

pub(super) fn grid_geometry(width: f32, depth: f32, options: GridFloorOptions) -> GeometryDesc {
    let spacing = options.line_spacing.max(0.001);
    let x_divisions = (width / spacing).round().clamp(1.0, 256.0) as u32;
    let z_divisions = (depth / spacing).round().clamp(1.0, 256.0) as u32;
    let half_width = width * 0.5;
    let half_depth = depth * 0.5;
    let normal = Vec3::Y;
    let mut vertices = Vec::with_capacity(((x_divisions + z_divisions + 2) * 2) as usize);
    let mut indices = Vec::with_capacity(vertices.capacity());

    for index in 0..=z_divisions {
        let z = -half_depth + depth * index as f32 / z_divisions as f32;
        push_grid_line(
            &mut vertices,
            &mut indices,
            Vec3::new(-half_width, 0.0, z),
            Vec3::new(half_width, 0.0, z),
            normal,
        );
    }
    for index in 0..=x_divisions {
        let x = -half_width + width * index as f32 / x_divisions as f32;
        push_grid_line(
            &mut vertices,
            &mut indices,
            Vec3::new(x, 0.0, -half_depth),
            Vec3::new(x, 0.0, half_depth),
            normal,
        );
    }

    GeometryDesc::try_new(GeometryTopology::Lines, vertices, indices)
        .expect("grid line indices are generated in pairs")
}

fn push_grid_line(
    vertices: &mut Vec<GeometryVertex>,
    indices: &mut Vec<u32>,
    start: Vec3,
    end: Vec3,
    normal: Vec3,
) {
    let base = vertices.len() as u32;
    vertices.push(GeometryVertex {
        position: start,
        normal,
    });
    vertices.push(GeometryVertex {
        position: end,
        normal,
    });
    indices.extend_from_slice(&[base, base + 1]);
}
