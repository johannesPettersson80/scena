use crate::{Color, GeometryDesc, GeometryTopology, GeometryVertex, Quat, Vec3};

/// Fraction of `extent` where the flat floor hands off to the sweep.
///
/// The final-photo subject floor is at least 1.8 subject radii wide, so 0.58
/// keeps the curve outside the product footprint while leaving room for a broad
/// studio transition.
pub(super) const BACKDROP_CURVE_START_FRACTION: f32 = 0.58;
/// Radius of the quarter-circle sweep as a fraction of `extent`.
///
/// A tight 0.18-radius bend compressed the complete normal transition into a
/// visible wall-floor line. A 0.30-radius sweep remains compact but reads as a
/// continuous cyclorama at native final resolution.
pub(super) const BACKDROP_CURVE_RADIUS_FRACTION: f32 = 0.50;
/// Fraction of `extent` the backdrop wall sits behind the subject.
pub(super) const BACKDROP_DEPTH_FRACTION: f32 =
    BACKDROP_CURVE_START_FRACTION + BACKDROP_CURVE_RADIUS_FRACTION;
/// Half-width of the receiver relative to its depth/height scale.
///
/// A studio sweep is wider than it is deep. Coupling both dimensions 1:1
/// makes a broad sweep impossible to cover at supported wide-angle framings:
/// increasing `extent` moves the wall away faster than it adds usable width.
pub(super) const BACKDROP_HALF_WIDTH_FRACTION: f32 = 1.8;
/// How tall the backdrop wall stands above its floor, as a fraction of `extent`.
/// This is the height `cyclorama_geometry` builds, and the vertical half of the
/// coverage solve has to agree with it or the wall's top edge enters the frame.
pub(super) const BACKDROP_WALL_HEIGHT_FRACTION: f32 = 1.8;
/// Extra wall height beyond the exact frustum solution.
///
/// Final reconstruction samples around nominal pixel centres, and composition
/// retries can differ by sub-pixel amounts. Geometry overscan keeps the wall edge
/// outside that footprint without moving the complete backdrop farther away.
pub(super) const BACKDROP_WALL_OVERSCAN_FRACTION: f32 = 0.30;
/// Slack beyond exact coverage, so the edge stays out of frame under the small
/// camera adjustments still made after the backdrop is sized.
pub(super) const BACKDROP_FRUSTUM_MARGIN: f32 = 1.12;
/// Fixed-point steps for the coverage solve.
const BACKDROP_SOLVE_STEPS: usize = 96;
pub(super) const CYC_FLOOR_SUBDIVISIONS: usize = 192;
pub(super) const CYC_ARC_SEGMENTS: usize = 512;
const CYC_FLOOR_CENTER_DENSITY_EXPONENT: f32 = 1.65;

/// The camera the backdrop has to satisfy.
#[derive(Clone, Copy)]
pub(super) struct BackdropCamera {
    pub(super) position: Vec3,
    pub(super) rotation: Quat,
    pub(super) vertical_fov: f32,
    pub(super) aspect: f32,
}

/// Where the backdrop stands, in its own basis.
#[derive(Clone, Copy)]
pub(super) struct BackdropPlane {
    pub(super) center: Vec3,
    /// Unit vector from the subject toward the camera; the wall's face normal.
    pub(super) toward_camera: Vec3,
    /// Unit vector across the wall; `half_width` is measured along this.
    pub(super) right: Vec3,
    /// World height of the wall's base.
    pub(super) floor_y: f32,
}

/// Size the floor and backdrop from what the camera can actually see.
///
/// This used to be `radius * 5` floored by `camera_distance * 1.35`, which is
/// unrelated to the frame. It only ever erred large, and a backdrop pushed
/// metres behind a subject photographed from centimetres away falls outside the
/// light rig entirely: measured on a travel mug, the rendered backdrop sat at a
/// median of 6.7/255 - black - against a 0.133 albedo, because inverse-square
/// falloff had nothing left to give at that distance. Covering the frame is a
/// property of the frustum, so it is computed from the frustum, and the subject
/// radius is a floor rather than the driver.
///
/// The first version of that solve took `max(tan(half_v), tan(half_h))` as one
/// half-angle and required `extent >= wall_distance * that`. It describes a
/// camera with no pitch, aimed along the wall's normal at the subject centre,
/// against a wall of unbounded height - and none of the three hold. The
/// camera-behavior loop pitches the camera down and re-aims it off the subject
/// centre to compose the frame, and `cyclorama_geometry` builds a wall only
/// `BACKDROP_WALL_HEIGHT_FRACTION * extent` tall. Checking neither the vertical
/// span nor the aim offset left the frame corner uncovered: measured on the demo
/// hero, a 191x28 px wedge of void in the top-left corner.
///
/// So the frustum's corner rays are projected onto the wall plane and the wall
/// is required to contain every hit, vertically as well as across. `extent`
/// appears on both sides - the wall stands `BACKDROP_DEPTH_FRACTION` of an
/// extent further back as it grows - so it is solved by fixed-point iteration.
pub(super) fn surroundings_extent(
    radius: f32,
    camera_distance: f32,
    half_extent_max: f32,
    plane: BackdropPlane,
    camera: Option<BackdropCamera>,
) -> f32 {
    // Enough to contain the subject and its floor contact regardless of framing.
    let subject_floor = radius.max(half_extent_max).mul_add(1.6, radius * 0.2);
    let Some(camera) = camera else {
        return subject_floor.max(camera_distance * 0.9);
    };
    let Some(corner_rays) = frustum_corner_rays(camera) else {
        return subject_floor.max(camera_distance * 0.9);
    };

    let mut extent = subject_floor.max(camera_distance * 0.5);
    for _ in 0..BACKDROP_SOLVE_STEPS {
        let Some(required) = required_extent_for(extent, plane, camera, &corner_rays) else {
            // Every corner ray runs parallel to the wall or behind the camera,
            // so the frustum never meets it and there is nothing to cover.
            return extent.max(subject_floor);
        };
        let next = required.max(subject_floor);
        if (next - extent).abs() <= extent * 1.0e-4 {
            extent = next;
            break;
        }
        extent = next;
    }
    extent.max(subject_floor)
}

/// The four corner directions of the frustum, in world space.
fn frustum_corner_rays(camera: BackdropCamera) -> Option<[Vec3; 4]> {
    let half_vertical = (camera.vertical_fov * 0.5).clamp(0.001, 1.5);
    let tan_vertical = half_vertical.tan();
    let tan_horizontal = tan_vertical * camera.aspect.max(0.05);
    if !tan_vertical.is_finite() || !tan_horizontal.is_finite() {
        return None;
    }
    // Camera local space is -Z forward, +Y up, +X right, matching the frustum
    // the inspection builder reports.
    Some([
        camera.rotation * Vec3::new(-tan_horizontal, tan_vertical, -1.0),
        camera.rotation * Vec3::new(tan_horizontal, tan_vertical, -1.0),
        camera.rotation * Vec3::new(-tan_horizontal, -tan_vertical, -1.0),
        camera.rotation * Vec3::new(tan_horizontal, -tan_vertical, -1.0),
    ])
}

/// Smallest `extent` whose wall contains every corner hit, given a wall placed
/// for `extent_guess`. Returns `None` when no corner ray reaches the wall.
fn required_extent_for(
    extent_guess: f32,
    plane: BackdropPlane,
    camera: BackdropCamera,
    corner_rays: &[Vec3; 4],
) -> Option<f32> {
    let wall_center = plane.center - plane.toward_camera * (BACKDROP_DEPTH_FRACTION * extent_guess);
    let mut required = 0.0_f32;
    let mut any_hit = false;
    for ray in corner_rays {
        let denominator = ray.dot(plane.toward_camera);
        // A ray heading along the wall's outward normal is travelling toward the
        // camera and never reaches it.
        if denominator >= -1.0e-6 {
            continue;
        }
        let distance = (wall_center - camera.position).dot(plane.toward_camera) / denominator;
        if !(distance.is_finite() && distance > 0.0) {
            continue;
        }
        any_hit = true;
        let hit = camera.position + *ray * distance;
        let offset = hit - wall_center;
        // Across: the receiver is deliberately wider than its depth scale.
        required = required.max(offset.dot(plane.right).abs() / BACKDROP_HALF_WIDTH_FRACTION);
        // Up: the wall spans BACKDROP_WALL_HEIGHT_FRACTION * extent above its
        // base. Below the base is the floor plane's job, not the wall's.
        let above_floor = hit.y - plane.floor_y;
        if above_floor > 0.0 {
            required = required.max(above_floor / BACKDROP_WALL_HEIGHT_FRACTION);
        }
    }
    any_hit.then_some(required * BACKDROP_FRUSTUM_MARGIN)
}

/// The wall's face direction: horizontal, from the subject toward the camera.
///
/// Shared with the coverage solve so the size the backdrop is solved for and the
/// basis it is built in cannot drift apart.
pub(super) fn horizontal_toward_camera(center: Vec3, camera_position: Vec3) -> Vec3 {
    let toward_camera = (camera_position - center).with_y(0.0).normalize_or_zero();
    if toward_camera.length_squared() > 1.0e-8 {
        toward_camera
    } else {
        Vec3::Z
    }
}

pub(super) fn cyclorama_geometry(
    center: Vec3,
    camera_position: Vec3,
    support_height: f32,
    extent: f32,
    include_floor: bool,
) -> GeometryDesc {
    let toward_camera = horizontal_toward_camera(center, camera_position);
    let away = -toward_camera;
    let right = Vec3::Y.cross(away).normalize_or_zero();
    let half_width = extent * BACKDROP_HALF_WIDTH_FRACTION;
    let curve_radius = extent * BACKDROP_CURVE_RADIUS_FRACTION;
    let curve_start = center + away * extent * BACKDROP_CURVE_START_FRACTION;
    let columns = if include_floor {
        CYC_FLOOR_SUBDIVISIONS + 1
    } else {
        2
    };
    let floor_rows = if include_floor {
        CYC_FLOOR_SUBDIVISIONS + 1
    } else {
        0
    };
    let rows = floor_rows + CYC_ARC_SEGMENTS + 1 + usize::from(!include_floor);
    let mut vertices = Vec::with_capacity(rows * columns);

    if include_floor {
        for depth_index in 0..=CYC_FLOOR_SUBDIVISIONS {
            let depth = cyclorama_floor_depth(depth_index, extent);
            let row_center = center + toward_camera * depth + Vec3::Y * (support_height - center.y);
            push_cyclorama_row(
                &mut vertices,
                row_center,
                Vec3::Y,
                right,
                half_width,
                columns,
            );
        }
    }

    let first_arc_segment = usize::from(include_floor);
    for segment in first_arc_segment..=CYC_ARC_SEGMENTS {
        let angle = segment as f32 / CYC_ARC_SEGMENTS as f32 * std::f32::consts::FRAC_PI_2;
        let row_center = curve_start
            + away * (curve_radius * angle.sin())
            + Vec3::Y * (support_height - center.y + curve_radius * (1.0 - angle.cos()));
        // The sweep's tangent is `away * cos + Y * sin`, so the face normal is
        // that rotated a quarter turn: `toward_camera * sin + Y * cos`. Emitting
        // the two terms the other way round leaves the sweep's floor end facing
        // sideways and its wall end facing straight up, which is a maximal normal
        // discontinuity against the flat floor and the flat wall it joins - a
        // shading seam at both ends that no amount of resizing can close.
        let normal = (toward_camera * angle.sin() + Vec3::Y * angle.cos()).normalize_or_zero();
        push_cyclorama_row(
            &mut vertices,
            row_center,
            normal,
            right,
            half_width,
            columns,
        );
    }
    let wall_center = curve_start
        + away * curve_radius
        + Vec3::Y
            * (support_height - center.y
                + extent * (BACKDROP_WALL_HEIGHT_FRACTION + BACKDROP_WALL_OVERSCAN_FRACTION));
    push_cyclorama_row(
        &mut vertices,
        wall_center,
        toward_camera,
        right,
        half_width,
        columns,
    );

    debug_assert_eq!(vertices.len(), rows * columns);
    let mut indices = Vec::with_capacity((rows - 1) * (columns - 1) * 6);
    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            let current = (row * columns + column) as u32;
            let next_row = current + columns as u32;
            indices.extend_from_slice(&[
                current,
                next_row,
                current + 1,
                current + 1,
                next_row,
                next_row + 1,
            ]);
        }
    }
    let tex_coords = vertices
        .iter()
        .map(|vertex| {
            let offset = vertex.position - center;
            let u = offset.dot(right) / (half_width * 2.0) + 0.5;
            let floor_v = (offset.dot(toward_camera) / extent + 1.0) * 0.25;
            let rise_v = ((vertex.position.y - support_height) / extent).max(0.0) * 0.5;
            [u, floor_v + rise_v]
        })
        .collect::<Vec<_>>();
    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        vertices.clone(),
        indices,
        vec![Color::WHITE; vertices.len()],
        tex_coords,
    )
    .expect("generated cyclorama geometry is valid")
}

/// Flat rear wall placed just in front of the lit cyclorama wall.
///
/// The floor and curved sweep must remain PBR receivers for physical area
/// shadows. The flat rear wall has a different job: provide a seamless neutral
/// background without showing the studio's emitters as a row of bright discs.
/// Keeping this cover separate avoids disabling useful reflections on the
/// product or useful shadowing on the floor.
pub(super) fn cyclorama_wall_cover_geometry(
    center: Vec3,
    camera_position: Vec3,
    support_height: f32,
    extent: f32,
) -> GeometryDesc {
    let toward_camera = horizontal_toward_camera(center, camera_position);
    let away = -toward_camera;
    let right = Vec3::Y.cross(away).normalize_or_zero();
    let half_width = extent * BACKDROP_HALF_WIDTH_FRACTION;
    let curve_radius = extent * BACKDROP_CURVE_RADIUS_FRACTION;
    let curve_start = center + away * extent * BACKDROP_CURVE_START_FRACTION;
    let wall_depth = curve_start + away * curve_radius;
    let lower = wall_depth
        + Vec3::Y * (support_height - center.y + curve_radius)
        + toward_camera * (extent * 1.0e-4).max(1.0e-5);
    let upper = wall_depth
        + Vec3::Y
            * (support_height - center.y
                + extent * (BACKDROP_WALL_HEIGHT_FRACTION + BACKDROP_WALL_OVERSCAN_FRACTION))
        + toward_camera * (extent * 1.0e-4).max(1.0e-5);
    let vertices = vec![
        GeometryVertex {
            position: lower - right * half_width,
            normal: toward_camera,
        },
        GeometryVertex {
            position: lower + right * half_width,
            normal: toward_camera,
        },
        GeometryVertex {
            position: upper - right * half_width,
            normal: toward_camera,
        },
        GeometryVertex {
            position: upper + right * half_width,
            normal: toward_camera,
        },
    ];
    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        vertices,
        vec![0, 2, 1, 1, 2, 3],
        vec![Color::WHITE; 4],
        vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
    )
    .expect("generated cyclorama wall cover geometry is valid")
}

fn push_cyclorama_row(
    vertices: &mut Vec<GeometryVertex>,
    center: Vec3,
    normal: Vec3,
    right: Vec3,
    half_width: f32,
    columns: usize,
) {
    for column in 0..columns {
        let across = if columns == 2 {
            column as f32 * 2.0 - 1.0
        } else {
            let unit = column as f32 / (columns - 1) as f32 * 2.0 - 1.0;
            unit.signum() * unit.abs().powf(CYC_FLOOR_CENTER_DENSITY_EXPONENT)
        };
        vertices.push(GeometryVertex {
            position: center + right * (across * half_width),
            normal,
        });
    }
}

fn cyclorama_floor_depth(index: usize, extent: f32) -> f32 {
    let midpoint = CYC_FLOOR_SUBDIVISIONS / 2;
    if index <= midpoint {
        let distance_from_center = (midpoint - index) as f32 / midpoint as f32;
        extent * distance_from_center.powf(CYC_FLOOR_CENTER_DENSITY_EXPONENT)
    } else {
        let distance_from_center =
            (index - midpoint) as f32 / (CYC_FLOOR_SUBDIVISIONS - midpoint) as f32;
        -extent
            * BACKDROP_CURVE_START_FRACTION
            * distance_from_center.powf(CYC_FLOOR_CENTER_DENSITY_EXPONENT)
    }
}
