use super::backdrop::{
    BACKDROP_DEPTH_FRACTION, BACKDROP_FRUSTUM_MARGIN, BACKDROP_HALF_WIDTH_FRACTION,
    BACKDROP_WALL_HEIGHT_FRACTION, CYC_ARC_SEGMENTS, CYC_FLOOR_SUBDIVISIONS,
};
use super::*;
use crate::GeometryDesc;

fn luminance(color: Color) -> f32 {
    Vec3::new(color.r, color.g, color.b).dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

#[test]
fn default_generated_floor_has_subtle_matte_normal_structure() {
    let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
    let geometry = host
        .assets
        .create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.4));
    let material = host
        .assets
        .create_material(MaterialDesc::pbr_metallic_roughness(
            Color::from_srgb_u8(88, 104, 122),
            0.72,
            0.28,
        ));
    let subject = host
        .scene
        .mesh(geometry, material)
        .add()
        .expect("subject mesh inserts");
    let subject = host.register_node(subject);

    let report = host
        .apply_photographic_surroundings(subject)
        .expect("surroundings solve");
    let receiver = host
        .resolve_node(report.support_nodes[0])
        .expect("generated receiver resolves");
    let crate::NodeKind::Mesh(receiver) =
        host.scene.node(receiver).expect("receiver remains").kind()
    else {
        panic!("generated receiver must remain a mesh");
    };
    let material = host
        .assets
        .material(receiver.material())
        .expect("receiver material resolves");
    assert!(material.roughness_factor() >= 0.9);
    let normal = material
        .normal_texture()
        .expect("matte floor needs bounded normal structure rather than a perfectly smooth sheet");
    let normal = host
        .assets
        .texture(normal)
        .expect("normal texture resolves");
    let (_, _, pixels) = normal.decoded_rgba8().expect("normal pixels decode");
    assert!(
        pixels
            .chunks_exact(4)
            .any(|pixel| pixel != [128, 128, 255, 255]),
        "matte normal texture must contain subtle deterministic variation"
    );
}

#[test]
fn reflective_ground_uses_one_non_recursive_mirrored_camera_capture() {
    let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
    let geometry = host
        .assets
        .create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.4));
    let material = host
        .assets
        .create_material(MaterialDesc::pbr_metallic_roughness(
            Color::from_srgb_u8(88, 104, 122),
            0.72,
            0.28,
        ));
    let subject = host
        .scene
        .mesh(geometry, material)
        .add()
        .expect("subject mesh inserts");
    let subject = host.register_node(subject);
    let original_camera = host.scene.active_camera();

    let mut report = host
        .apply_photographic_surroundings_with_ground(subject, PhotographicGroundV1::Reflective)
        .expect("reflective surroundings solve");
    assert_eq!(report.ground, PhotographicGroundV1::Reflective);
    assert!(report.reflection_strength > 0.0);
    assert!(report.reflection_roughness > 0.0);
    assert!(host.renderer.screen_space_reflections().is_none());

    let capture = host
        .capture_photographic_planar_reflection(&mut report)
        .expect("planar reflection capture succeeds")
        .expect("reflective ground returns one capture");
    assert_eq!(capture.capture_count, 1);
    assert!(capture.excluded_floor_nodes >= 1);
    assert_eq!(report.planar_reflection_capture_count, 1);
    assert_eq!(host.scene.active_camera(), original_camera);
    assert!(
        report.support_nodes.iter().all(|handle| host
            .resolve_node(*handle)
            .ok()
            .and_then(|node| host.scene.visible(node))
            == Some(true)),
        "floor visibility must be restored after the mirrored capture"
    );
    assert!(host.renderer.screen_space_reflections().is_none());
}

#[test]
fn generated_floor_has_enough_vertices_to_receive_local_area_shadows() {
    let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
    let geometry = host
        .assets
        .create_geometry(GeometryDesc::box_xyz(2.0, 1.0, 1.2));
    let material = host
        .assets
        .create_material(MaterialDesc::pbr_metallic_roughness(
            Color::from_srgb_u8(88, 104, 122),
            0.72,
            0.28,
        ));
    let subject = host
        .scene
        .mesh(geometry, material)
        .add()
        .expect("subject mesh inserts");
    let subject = host.register_node(subject);

    let report = host
        .apply_photographic_surroundings(subject)
        .expect("surroundings solve");
    let floor = host
        .resolve_node(report.generated_nodes[0])
        .expect("generated floor resolves");
    let crate::NodeKind::Mesh(floor) = host.scene.node(floor).expect("floor node remains").kind()
    else {
        panic!("first generated surrounding is the floor");
    };
    let floor = host
        .assets
        .geometry(floor.geometry())
        .expect("floor geometry remains");

    assert!(
        floor.vertices().len() >= 1_000,
        "area-light visibility is baked per vertex, so a four-corner floor cannot resolve a \
         product-sized cast shadow; vertices={}",
        floor.vertices().len()
    );
    let footprint_vertices = floor
        .vertices()
        .iter()
        .filter(|vertex| vertex.position.x.abs() <= 1.0 && vertex.position.z.abs() <= 0.6)
        .count();
    assert!(
        footprint_vertices >= 100,
        "the dense receiver region must cover the subject footprint; local vertices={footprint_vertices}"
    );
}

#[test]
fn generated_floor_keeps_microstructure_off_the_smooth_pbr_backdrop() {
    let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
    let geometry = host
        .assets
        .create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.4));
    let material = host
        .assets
        .create_material(MaterialDesc::pbr_metallic_roughness(
            Color::from_srgb_u8(88, 104, 122),
            0.72,
            0.28,
        ));
    let subject = host
        .scene
        .mesh(geometry, material)
        .add()
        .expect("subject mesh inserts");
    let subject = host.register_node(subject);

    let report = host
        .apply_photographic_surroundings(subject)
        .expect("surroundings solve");
    assert!(report.generated_floor);
    assert!(report.generated_cyclorama);
    assert_eq!(
        report.support_nodes.len(),
        1,
        "the report identifies the generated floor independently from backdrop and contact proxies",
    );
    assert_eq!(
        report.backdrop_nodes.len(),
        2,
        "the report identifies one smooth curved sweep and one smooth flat wall",
    );
    assert_ne!(
        report.support_nodes, report.backdrop_nodes,
        "the textured floor must stay independent from the smooth studio backdrop"
    );

    let node = host
        .resolve_node(report.support_nodes[0])
        .expect("generated receiver resolves");
    let crate::NodeKind::Mesh(mesh) = host
        .scene
        .node(node)
        .expect("generated receiver remains")
        .kind()
    else {
        panic!("generated receiver must remain a mesh");
    };
    let receiver = host
        .assets
        .geometry(mesh.geometry())
        .expect("generated receiver geometry remains");
    let floor_half_span = receiver
        .vertices()
        .iter()
        .map(|vertex| vertex.position.x.abs().max(vertex.position.z.abs()))
        .fold(0.0_f32, f32::max);
    assert!(
        floor_half_span >= report.extent_m * BACKDROP_HALF_WIDTH_FRACTION * 0.999,
        "the floor must cover the same wide framing envelope as the backdrop; \
         half_span={floor_half_span}, extent={}",
        report.extent_m
    );
    assert!(
        receiver
            .vertices()
            .iter()
            .all(|vertex| vertex.normal.dot(Vec3::Y) > 0.999),
        "the textured matte receiver must be the near floor only"
    );
    let toward_camera = Vec3::Z;
    let flat_floor_depths = receiver
        .vertices()
        .iter()
        .filter(|vertex| vertex.normal.dot(Vec3::Y) > 0.999)
        .map(|vertex| vertex.position.dot(toward_camera))
        .collect::<Vec<_>>();
    let floor_front = flat_floor_depths
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let floor_back = flat_floor_depths
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    assert!(
        floor_front > 0.5 && floor_back < -0.25,
        "the continuous receiver must contain flat floor on both sides of the subject before \
         rising into the sweep; depth range={floor_back}..{floor_front}"
    );

    let sweep = host
        .resolve_node(report.backdrop_nodes[0])
        .expect("generated sweep resolves");
    let crate::NodeKind::Mesh(sweep) = host
        .scene
        .node(sweep)
        .expect("generated sweep remains")
        .kind()
    else {
        panic!("generated sweep must remain a mesh");
    };
    let sweep_material = host
        .assets
        .material(sweep.material())
        .expect("generated sweep material remains");
    assert_eq!(
        sweep_material.kind(),
        crate::MaterialKind::PbrMetallicRoughness,
        "seamless studio paper should retain physically consistent light falloff"
    );
    assert_eq!(
        sweep_material.normal_texture(),
        None,
        "the curved sweep must be optically smooth instead of repeating the floor micro-normal"
    );

    let backdrop = host
        .resolve_node(report.backdrop_nodes[1])
        .expect("generated rear wall resolves");
    let crate::NodeKind::Mesh(backdrop) = host
        .scene
        .node(backdrop)
        .expect("generated rear wall remains")
        .kind()
    else {
        panic!("generated rear wall must remain a mesh");
    };
    let backdrop_material = host
        .assets
        .material(backdrop.material())
        .expect("generated rear-wall material remains");
    assert_eq!(
        backdrop_material.kind(),
        crate::MaterialKind::PbrMetallicRoughness,
        "the flat wall should match the physically lit seamless sweep"
    );
    assert_eq!(
        backdrop_material.normal_texture(),
        None,
        "the rear wall must not inherit the floor micro-normal"
    );
}

/// Subject centred on the origin, sitting on a floor at `-radius`.
fn test_plane(center: Vec3, radius: f32, camera_position: Vec3) -> BackdropPlane {
    let toward_camera = horizontal_toward_camera(center, camera_position);
    BackdropPlane {
        center,
        toward_camera,
        right: Vec3::Y.cross(-toward_camera).normalize_or_zero(),
        floor_y: center.y - radius - radius * 0.7,
    }
}

/// Where the frame's corners actually land on the wall, in the wall's basis.
///
/// This is deliberately an independent projection rather than a call into
/// `required_extent_for`: a test that reuses the solver's own arithmetic
/// cannot catch the solver being wrong about the geometry, which is exactly
/// the failure this replaces.
fn worst_corner_hit(extent: f32, plane: BackdropPlane, camera: BackdropCamera) -> (f32, f32) {
    let half_vertical = camera.vertical_fov * 0.5;
    let tan_vertical = half_vertical.tan();
    let tan_horizontal = tan_vertical * camera.aspect;
    let wall_center = plane.center - plane.toward_camera * (BACKDROP_DEPTH_FRACTION * extent);
    let mut widest = 0.0_f32;
    let mut highest = 0.0_f32;
    for (sx, sy) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let ray = camera.rotation * Vec3::new(sx * tan_horizontal, sy * tan_vertical, -1.0);
        let denominator = ray.dot(plane.toward_camera);
        if denominator >= -1.0e-6 {
            continue;
        }
        let distance = (wall_center - camera.position).dot(plane.toward_camera) / denominator;
        if !(distance.is_finite() && distance > 0.0) {
            continue;
        }
        let hit = camera.position + ray * distance;
        widest = widest.max((hit - wall_center).dot(plane.right).abs());
        highest = highest.max(hit.y - plane.floor_y);
    }
    (widest, highest)
}

/// The backdrop has two jobs that pull against each other: it must cover the
/// frame, and it must stay close enough to the light rig to be lit.
///
/// The sweep now includes pitched and off-centre cameras, because the
/// camera-behavior loop produces both and the previous version of this test
/// modelled neither - it asserted a head-on, unpitched, subject-centred
/// camera and so passed while the demo hero rendered a wedge of void in its
/// top-left corner.
#[test]
fn backdrop_covers_the_frame_without_retreating_out_of_the_light_rig() {
    for &focal_mm in &[35.0_f32, 58.0, 85.0, 135.0] {
        // Vertical FOV of a full-frame sensor at this focal length.
        let vertical_fov = 2.0 * (12.0_f32 / focal_mm).atan();
        for &aspect in &[1.0_f32, 1.524, 1.778] {
            for &radius in &[0.05_f32, 0.2, 0.54, 2.0] {
                for &distance in &[radius * 2.0, radius * 4.0, radius * 8.0] {
                    for &pitch in &[0.0_f32, 0.30, 0.52] {
                        for &aim_offset in &[0.0_f32, 0.18, -0.35] {
                            let center = Vec3::ZERO;
                            let camera_position = Vec3::new(
                                distance * pitch.cos() * 0.6,
                                distance * pitch.sin(),
                                distance * pitch.cos() * 0.8,
                            );
                            let plane = test_plane(center, radius, camera_position);
                            // The composition corrector aims off the subject
                            // centre to place it in the frame; the backdrop
                            // has to cover where the camera actually looks.
                            let aim = center + plane.right * (aim_offset * radius);
                            let camera = BackdropCamera {
                                position: camera_position,
                                rotation: Transform::at(camera_position)
                                    .looking_at(aim, Vec3::Y)
                                    .rotation,
                                vertical_fov,
                                aspect,
                            };
                            let extent = surroundings_extent(
                                radius,
                                distance,
                                radius * 0.8,
                                plane,
                                Some(camera),
                            );

                            // Coverage, measured the way the renderer sees
                            // it: every frame corner must land on the wall,
                            // across *and* up.
                            let (widest, highest) = worst_corner_hit(extent, plane, camera);
                            let half_width = extent * BACKDROP_HALF_WIDTH_FRACTION;
                            assert!(
                                half_width >= widest * 0.999,
                                "backdrop half-width {half_width} does not reach the {widest} m \
                                 frame corner (focal {focal_mm}mm, aspect {aspect}, \
                                 radius {radius}, distance {distance}, pitch {pitch}, \
                                 aim {aim_offset})"
                            );
                            assert!(
                                extent * BACKDROP_WALL_HEIGHT_FRACTION >= highest * 0.999,
                                "backdrop wall {} m tall does not reach the {highest} m frame \
                                 corner (focal {focal_mm}mm, aspect {aspect}, \
                                 radius {radius}, distance {distance}, pitch {pitch}, \
                                 aim {aim_offset})",
                                extent * BACKDROP_WALL_HEIGHT_FRACTION
                            );

                            // Reach: it must also contain the subject, and
                            // must not run away from the lights the way
                            // `radius * 5` did.
                            assert!(
                                extent >= radius,
                                "backdrop {extent} is smaller than the subject radius {radius}"
                            );
                            let subject_floor = radius.max(radius * 0.8) * 1.6 + radius * 0.2;
                            let needed = (widest * BACKDROP_FRUSTUM_MARGIN
                                / BACKDROP_HALF_WIDTH_FRACTION)
                                .max(
                                    highest * BACKDROP_FRUSTUM_MARGIN
                                        / BACKDROP_WALL_HEIGHT_FRACTION,
                                )
                                .max(subject_floor);
                            assert!(
                                extent <= needed * 1.05,
                                "backdrop {extent} is larger than the {needed} it takes to \
                                 cover the frame (radius {radius}, distance {distance}, \
                                 focal {focal_mm}mm, aspect {aspect}, pitch {pitch}, \
                                 aim {aim_offset})"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Negative control: the retired solve must actually fail the space the new
/// one covers, or the fix above is proving nothing.
///
/// The retired rule solved `extent = distance * tan / (1 - depth * tan)`
/// from a single half-angle, which describes an unpitched camera aimed along
/// the wall normal at the subject centre, against a wall of unbounded height.
/// This walks the same framings the coverage sweep does and counts how many
/// leave a frame corner off the wall under that rule. The demo hero rendered
/// exactly this defect: a 191x28 px wedge of void in its top-left corner.
#[test]
fn the_retired_width_only_solve_leaves_frame_corners_off_the_backdrop() {
    let mut uncovered = 0_usize;
    let mut total = 0_usize;
    for &focal_mm in &[35.0_f32, 58.0, 85.0, 135.0] {
        let vertical_fov = 2.0 * (12.0_f32 / focal_mm).atan();
        for &aspect in &[1.0_f32, 1.524, 1.778] {
            for &radius in &[0.05_f32, 0.2, 0.54, 2.0] {
                for &distance in &[radius * 2.0, radius * 4.0, radius * 8.0] {
                    for &pitch in &[0.0_f32, 0.30, 0.52] {
                        for &aim_offset in &[0.0_f32, 0.18, -0.35] {
                            let center = Vec3::ZERO;
                            let camera_position = Vec3::new(
                                distance * pitch.cos() * 0.6,
                                distance * pitch.sin(),
                                distance * pitch.cos() * 0.8,
                            );
                            let plane = test_plane(center, radius, camera_position);
                            let aim = center + plane.right * (aim_offset * radius);
                            let camera = BackdropCamera {
                                position: camera_position,
                                rotation: Transform::at(camera_position)
                                    .looking_at(aim, Vec3::Y)
                                    .rotation,
                                vertical_fov,
                                aspect,
                            };

                            // The retired rule, reproduced exactly.
                            let half_vertical = vertical_fov * 0.5;
                            let tangent = half_vertical
                                .max((half_vertical.tan() * aspect).atan())
                                .tan()
                                * 1.3;
                            let denominator = 1.0 - BACKDROP_DEPTH_FRACTION * tangent;
                            let retired = if denominator > 0.2 {
                                distance * tangent / denominator
                            } else {
                                distance * 2.5
                            }
                            .max(radius.max(radius * 0.8) * 1.6 + radius * 0.2);

                            let (widest, highest) = worst_corner_hit(retired, plane, camera);
                            total += 1;
                            if retired < widest || retired * BACKDROP_WALL_HEIGHT_FRACTION < highest
                            {
                                uncovered += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        uncovered > 0,
        "the retired solve covered all {total} framings, so the coverage sweep above \
         would have passed before the fix and proves nothing"
    );
    // Measured 260 of 1296 framings at the time this was written. Held to a
    // looser floor than that so ordinary numeric drift does not fail the
    // test, but tight enough that a sweep quietly narrowed to head-on
    // cameras - where the retired rule was correct - shows up here rather
    // than silently turning the coverage test above into a tautology.
    assert!(
        uncovered * 8 >= total,
        "only {uncovered} of {total} framings expose the retired defect; the sweep no \
         longer covers the pitched, off-axis cameras this fix is about"
    );
}

/// An orthographic camera has no frustum divergence to solve, so the subject
/// floor governs rather than a bogus extrapolation.
#[test]
fn orthographic_and_unknown_cameras_fall_back_to_the_subject_floor() {
    let plane = test_plane(Vec3::ZERO, 0.5, Vec3::new(0.0, 0.5, 2.0));
    let extent = surroundings_extent(0.5, 2.0, 0.4, plane, None);
    assert!(
        extent >= 0.5,
        "fallback backdrop {extent} must still contain the subject"
    );
    assert!(
        extent <= 5.0,
        "fallback backdrop {extent} must stay bounded"
    );
}

/// The grading property, swept rather than spot-checked: a surround must
/// stay clear of black, stay separated from the subject, and never get
/// darker as the subject gets brighter.
#[test]
fn derived_surround_separates_from_the_subject_without_crushing() {
    let mut previous: Option<(f32, f32)> = None;
    let mut samples = 0;
    for step in 1..=95 {
        let subject_luminance = step as f32 / 100.0;
        let subject =
            Color::from_linear_rgb(subject_luminance, subject_luminance, subject_luminance);
        let surround = luminance(derived_background(subject, subject_luminance));

        assert!(
            surround >= MIN_SURROUND_LUMINANCE - 1.0e-4,
            "surround crushed to {surround} for subject {subject_luminance}"
        );
        let ratio = if surround >= subject_luminance {
            surround / subject_luminance.max(1.0e-5)
        } else {
            subject_luminance / surround.max(1.0e-5)
        };
        assert!(
            ratio >= 1.8,
            "surround {surround} is only {ratio}x from subject {subject_luminance}; \
             the subject will not separate from its background"
        );

        // Monotonic within a branch. The single flip from a lifted to a
        // dropped surround is a deliberate low-key/high-key decision.
        if let Some((previous_subject, previous_surround)) = previous
            && surround + 1.0e-4 < previous_surround
            && subject_luminance / SURROUND_SEPARATION >= MIN_SURROUND_LUMINANCE
            && previous_subject / SURROUND_SEPARATION >= MIN_SURROUND_LUMINANCE
        {
            panic!(
                "surround got darker ({previous_surround} -> {surround}) as the subject \
                 got brighter ({previous_subject} -> {subject_luminance})"
            );
        }
        previous = Some((subject_luminance, surround));
        samples += 1;
    }
    assert_eq!(samples, 95);
}

/// The seamless-sweep property, asserted on geometry rather than on pixels:
/// every emitted normal must agree with the surface the neighbouring rows
/// actually describe, and the two ends must match the flat floor and flat
/// wall they butt against.
#[test]
fn cyclorama_normals_match_the_surface_they_sweep() {
    let center = Vec3::new(0.0, 0.0, 0.0);
    let camera = Vec3::new(0.0, 1.4, 6.0);
    let toward_camera = Vec3::Z;
    let geometry = cyclorama_geometry(center, camera, 0.0, 2.0, true);
    let vertices = geometry.vertices();

    // The dense floor and sweep share a column count. The final row is the wall
    // cap, whose normal is authored flat, so compare curved rows only. Use a
    // central difference: on a circular arc the chord from `i - 1` to
    // `i + 1` is parallel to the tangent at `i` by symmetry, whereas a
    // forward difference is off by half a segment angle and would report a
    // correct sweep as broken.
    let columns = CYC_FLOOR_SUBDIVISIONS + 1;
    let rows = vertices.len() / columns;
    let first_arc_row = CYC_FLOOR_SUBDIVISIONS + 1;
    for row in first_arc_row..first_arc_row + CYC_ARC_SEGMENTS - 1 {
        let previous = vertices[(row - 1) * columns].position;
        let next = vertices[(row + 1) * columns].position;
        let tangent = (next - previous).normalize();
        let normal = vertices[row * columns].normal.normalize();
        assert!(
            normal.dot(tangent).abs() < 1.0e-3,
            "row {row} normal {normal:?} is not perpendicular to the surface \
             direction {tangent:?} it sweeps along"
        );
        assert!(
            normal.dot(toward_camera) >= -1.0e-4 && normal.dot(Vec3::Y) >= -1.0e-4,
            "row {row} normal {normal:?} must face the camera side, not away from it"
        );
    }

    let floor_end = vertices[CYC_FLOOR_SUBDIVISIONS * columns]
        .normal
        .normalize();
    assert!(
        floor_end.dot(Vec3::Y) > 0.999,
        "the sweep's floor end must match the floor's up normal, got {floor_end:?}"
    );
    let wall_end = vertices[(rows - 2) * columns].normal.normalize();
    assert!(
        wall_end.dot(toward_camera) > 0.999,
        "the sweep's wall end must match the wall's normal, got {wall_end:?}"
    );
    let wall_cap = vertices[(rows - 1) * columns].normal.normalize();
    assert!(
        wall_end.dot(wall_cap) > 0.999,
        "the sweep must hand off to the wall cap without a normal step"
    );
}

#[test]
fn cyclorama_arc_is_dense_enough_for_final_product_stills() {
    let geometry = cyclorama_geometry(Vec3::ZERO, Vec3::new(0.0, 1.4, 6.0), 0.0, 2.0, true);
    let columns = CYC_FLOOR_SUBDIVISIONS + 1;
    let arc_segments = geometry.vertices().len() / columns - columns - 1;

    assert!(
        arc_segments >= 512,
        "a 64-segment quarter sweep still produces visible horizontal facet bands at 4K; \
         final staging needs at least 512 arc segments, got {}",
        arc_segments
    );
}

#[test]
fn final_cyclorama_keeps_the_sweep_broad_and_the_wall_edge_out_of_frame() {
    let extent = 2.0;
    let support_height = 0.25;
    let geometry = cyclorama_geometry(
        Vec3::ZERO,
        Vec3::new(0.0, 1.4, 6.0),
        support_height,
        extent,
        true,
    );
    let columns = CYC_FLOOR_SUBDIVISIONS + 1;
    let vertices = geometry.vertices();
    let floor_end = vertices[CYC_FLOOR_SUBDIVISIONS * columns].position;
    let first_arc_row = CYC_FLOOR_SUBDIVISIONS + 1;
    let arc_end = vertices[(first_arc_row + CYC_ARC_SEGMENTS - 1) * columns].position;
    let wall_top = vertices[vertices.len() - columns].position;

    assert!(
        -floor_end.z >= extent * 0.55,
        "the sweep must start behind the product footprint; start depth={} extent={extent}",
        -floor_end.z
    );
    assert!(
        arc_end.y - support_height >= extent * 0.45,
        "the measured 30%-radius sweep still leaves a visible horizontal studio transition; \
         rise={} extent={extent}",
        arc_end.y - support_height
    );
    assert!(
        wall_top.y - support_height >= extent * (BACKDROP_WALL_HEIGHT_FRACTION + 0.25),
        "the generated wall needs geometric overscan beyond the exact frustum solve so its \
         top edge cannot enter a reconstructed final frame"
    );
}

#[test]
fn final_cyclorama_receiver_is_dense_enough_for_a_smooth_visibility_field() {
    const {
        assert!(
            CYC_FLOOR_SUBDIVISIONS >= 192,
            "a 192x192-or-denser receiver leaves prepared area-light visibility as visible \
             triangular patches under the valve plate at 4K"
        );
    }
    let geometry = cyclorama_geometry(Vec3::ZERO, Vec3::new(0.0, 1.4, 6.0), 0.0, 2.0, true);
    assert!(
        geometry.vertices().len() >= 50_000,
        "the final receiver must carry enough vertices to interpolate the prepared visibility \
         field without a polygonal smudge; vertices={}",
        geometry.vertices().len()
    );
}
