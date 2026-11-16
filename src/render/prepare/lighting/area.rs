use crate::material::Color;
use crate::scene::{AreaLight, AreaLightShape, Vec3, WorldTransform};

use super::math::{light_axis_direction, normalize_or, rotate_vec3, subtract_vec3};

pub(super) const AREA_LIGHT_SAMPLE_COUNT: usize = 16;
const FRAC_1_SQRT_2: f32 = std::f32::consts::FRAC_1_SQRT_2;

const RECT_SAMPLE_OFFSETS: [(f32, f32); AREA_LIGHT_SAMPLE_COUNT] = [
    (-0.75, -0.75),
    (-0.25, -0.75),
    (0.25, -0.75),
    (0.75, -0.75),
    (-0.75, -0.25),
    (-0.25, -0.25),
    (0.25, -0.25),
    (0.75, -0.25),
    (-0.75, 0.25),
    (-0.25, 0.25),
    (0.25, 0.25),
    (0.75, 0.25),
    (-0.75, 0.75),
    (-0.25, 0.75),
    (0.25, 0.75),
    (0.75, 0.75),
];

const DISC_SAMPLE_OFFSETS: [(f32, f32); AREA_LIGHT_SAMPLE_COUNT] = [
    (0.35, 0.0),
    (0.247487, 0.247487),
    (0.0, 0.35),
    (-0.247487, 0.247487),
    (-0.35, 0.0),
    (-0.247487, -0.247487),
    (0.0, -0.35),
    (0.247487, -0.247487),
    (0.69291, 0.287013),
    (0.287013, 0.69291),
    (-0.287013, 0.69291),
    (-0.69291, 0.287013),
    (-0.69291, -0.287013),
    (-0.287013, -0.69291),
    (0.287013, -0.69291),
    (0.69291, -0.287013),
];

const SPHERE_SAMPLE_OFFSETS: [(f32, f32, f32); AREA_LIGHT_SAMPLE_COUNT] = [
    (0.577350, 0.577350, 0.577350),
    (-0.577350, 0.577350, 0.577350),
    (0.577350, -0.577350, 0.577350),
    (-0.577350, -0.577350, 0.577350),
    (0.577350, 0.577350, -0.577350),
    (-0.577350, 0.577350, -0.577350),
    (0.577350, -0.577350, -0.577350),
    (-0.577350, -0.577350, -0.577350),
    (1.0, 0.0, 0.0),
    (-1.0, 0.0, 0.0),
    (0.0, 1.0, 0.0),
    (0.0, -1.0, 0.0),
    (0.0, 0.0, 1.0),
    (0.0, 0.0, -1.0),
    (0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (0.0, -FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
];

#[derive(Clone, Copy)]
pub(super) struct PreparedAreaLight {
    pub(super) color: Color,
    pub(super) position: Vec3,
    pub(super) axis_x: Vec3,
    pub(super) axis_y: Vec3,
    pub(super) luminous_flux_lumens: f32,
    pub(super) range: Option<f32>,
    pub(super) shape: AreaLightShape,
}

/// Places an area light with the exact world matrix of its node.
///
/// A rectangle or disc lies in the plane its local X and Y axes span, so its
/// two axes follow the matrix columns and stop being perpendicular when the
/// hierarchy shears. Their lengths stay the authored size: a node's scale
/// never resizes a light. A sphere has no orientation and keeps an
/// orthonormal frame, which leaves it a sphere of its authored radius.
pub(super) fn prepared_area_light(
    light: AreaLight,
    world: WorldTransform,
    origin_shift: Vec3,
) -> Option<PreparedAreaLight> {
    let luminous_flux_lumens = light.luminous_flux_lumens();
    if !luminous_flux_lumens.is_finite() || luminous_flux_lumens <= 0.0 {
        return None;
    }
    let shape = light.shape();
    let (right, up) = match shape {
        AreaLightShape::Rect { .. } | AreaLightShape::Disc { .. } => (
            light_axis_direction(world, Vec3::X),
            light_axis_direction(world, Vec3::Y),
        ),
        AreaLightShape::Sphere { .. } => (
            rotate_vec3(world.trs().rotation, Vec3::X),
            rotate_vec3(world.trs().rotation, Vec3::Y),
        ),
    };
    let (axis_x, axis_y) = area_light_axes(shape, right, up);
    Some(PreparedAreaLight {
        color: light.color(),
        position: subtract_vec3(world.translation(), origin_shift),
        axis_x,
        axis_y,
        luminous_flux_lumens,
        range: light.range(),
        shape,
    })
}

fn area_light_axes(shape: AreaLightShape, right: Vec3, up: Vec3) -> (Vec3, Vec3) {
    match shape {
        AreaLightShape::Rect { width, height } => (
            right * (width.max(0.001) * 0.5),
            up * (height.max(0.001) * 0.5),
        ),
        AreaLightShape::Disc { radius } => {
            let r = radius.max(0.001);
            (right * r, up * r)
        }
        AreaLightShape::Sphere { radius } => {
            let r = radius.max(0.001);
            (right * r, up * r)
        }
    }
}

pub(super) fn area_light_shape_code(shape: AreaLightShape) -> f32 {
    match shape {
        AreaLightShape::Rect { .. } => 0.0,
        AreaLightShape::Disc { .. } => 1.0,
        AreaLightShape::Sphere { .. } => 2.0,
    }
}

pub(super) fn area_light_sample_positions(
    light: PreparedAreaLight,
) -> [Vec3; AREA_LIGHT_SAMPLE_COUNT] {
    let mut positions = [light.position; AREA_LIGHT_SAMPLE_COUNT];
    match light.shape {
        AreaLightShape::Rect { .. } => {
            for (index, (x, y)) in RECT_SAMPLE_OFFSETS.iter().copied().enumerate() {
                positions[index] = light.position + light.axis_x * x + light.axis_y * y;
            }
        }
        AreaLightShape::Disc { .. } => {
            for (index, (x, y)) in DISC_SAMPLE_OFFSETS.iter().copied().enumerate() {
                positions[index] = light.position + light.axis_x * x + light.axis_y * y;
            }
        }
        AreaLightShape::Sphere { .. } => {
            let radius = light.axis_x.length().max(light.axis_y.length()).max(0.001);
            let forward = normalize_or(light.axis_x.cross(light.axis_y), Vec3::Z) * radius;
            for (index, (x, y, z)) in SPHERE_SAMPLE_OFFSETS.iter().copied().enumerate() {
                positions[index] =
                    light.position + light.axis_x * x + light.axis_y * y + forward * z;
            }
        }
    }
    positions
}
