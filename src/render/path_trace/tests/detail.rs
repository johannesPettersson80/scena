//! Rounded edges and surface finishes on scenes with known answers.

use super::super::detail::DetailSettings;
use super::super::rounding::SQUARE_EDGE;
use super::super::sampling::Rng;
use super::super::scene::{NONE, TraceScene};
use super::*;
use crate::{PhotographicSurfaceKind, SurfaceFinish};

fn traced(stage: &Stage) -> TraceScene {
    let detail = DetailSettings {
        enabled: true,
        edge_radius: 0.0,
    };
    TraceScene::build(&stage.scene, &stage.assets, Vec3::ZERO, &[], detail)
        .expect("the scene builds")
}

/// The rounded normal and the convexity where a ray meets the scene, from
/// the sums of many sets of probes, and the mean of the normals the sets
/// gave one by one.
fn rounded_at(scene: &TraceScene, from: Vec3, direction: Vec3, radius: f32) -> (Vec3, f32, Vec3) {
    const ROUNDS: u32 = 1500;
    let mut rng = Rng::new(7, 1);
    let hit = scene
        .closest(from, direction, f32::INFINITY, NONE, &mut rng)
        .expect("the ray meets a surface");
    let position = from + direction * hit.distance;
    let (winding, smooth) = scene.outward_normals(&hit);
    let (mut sum, mut total, mut fall, mut singly) = (Vec3::ZERO, 0.0, 0.0, Vec3::ZERO);
    for _ in 0..ROUNDS {
        let gathered = scene.gather(hit.triangle, position, winding, radius, &mut rng);
        sum += gathered.sum;
        total += gathered.total;
        fall += gathered.fall;
        singly += scene
            .rounded(hit.triangle, position, winding, smooth, radius, &mut rng)
            .normal;
    }
    (sum.normalize(), fall / total, singly.normalize())
}

/// A floor in `y = 0` and a wall in `x = 0` that meet along the z axis, as
/// one mesh. Both face the quarter of space with positive x and y.
fn floor_and_wall() -> GeometryDesc {
    let corner = |position: [f32; 3], normal: Vec3| GeometryVertex {
        position: Vec3::from(position),
        normal,
    };
    let vertices = vec![
        corner([0.0, 0.0, -2.0], Vec3::Y),
        corner([0.0, 0.0, 2.0], Vec3::Y),
        corner([2.0, 0.0, 2.0], Vec3::Y),
        corner([2.0, 0.0, -2.0], Vec3::Y),
        corner([0.0, 0.0, -2.0], Vec3::X),
        corner([0.0, 2.0, -2.0], Vec3::X),
        corner([0.0, 2.0, 2.0], Vec3::X),
        corner([0.0, 0.0, 2.0], Vec3::X),
    ];
    let indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices).expect("mesh builds")
}

#[test]
fn a_flat_face_keeps_its_normal_and_an_edge_leans_towards_its_neighbour() {
    let mut stage = Stage::top_down();
    stage.mesh(
        GeometryDesc::box_xyz(2.0, 2.0, 2.0),
        matte(0.8),
        Transform::IDENTITY,
    );
    let scene = traced(&stage);
    let radius = 0.1;
    let down = |x: f32| rounded_at(&scene, Vec3::new(x, 5.0, 0.13), Vec3::NEG_Y, radius);

    for inside in [0.0, 1.0 - 1.2 * radius] {
        let (normal, convexity, singly) = down(inside);
        assert!((normal - Vec3::Y).length() < 1.0e-5 && convexity.abs() < 1.0e-5);
        assert!(
            (singly - Vec3::Y).length() < 1.0e-5,
            "every set of probes agrees"
        );
    }

    // On the edge itself the surface is half one face and half the other.
    let (normal, convexity, singly) = down(1.0 - 0.01 * radius);
    assert!((normal.x - normal.y).abs() < 0.1, "{normal}");
    assert!(normal.z.abs() < 0.05 && normal.x > 0.6);
    assert!((convexity / SQUARE_EDGE - 1.0).abs() < 0.15, "{convexity}");
    // One set of probes is a rough estimate, but it leans the same way.
    assert!(
        singly.x > 0.35 && singly.y > 0.6 && singly.z.abs() < 0.05,
        "{singly}"
    );

    // Part of the way in, it leans part of the way, and the two faces
    // round the same.
    let (top, convexity, _) = down(1.0 - 0.4 * radius);
    assert!(top.x > 0.1 && top.x < 0.5 && top.y > 0.85, "{top}");
    assert!(convexity > 0.1 * SQUARE_EDGE && convexity < 0.8 * SQUARE_EDGE);
    let (side, ..) = rounded_at(
        &scene,
        Vec3::new(5.0, 1.0 - 0.4 * radius, 0.13),
        Vec3::NEG_X,
        radius,
    );
    assert!((side.x - top.y).abs() < 0.05 && (side.y - top.x).abs() < 0.05);

    // A corner turns towards all three faces.
    let (corner, convexity, _) = rounded_at(
        &scene,
        Vec3::new(1.0 - 0.05 * radius, 5.0, 1.0 - 0.05 * radius),
        Vec3::NEG_Y,
        radius,
    );
    assert!(corner.min_element() > 0.4, "{corner}");
    assert!(convexity > 1.2 * SQUARE_EDGE);
}

#[test]
fn an_inside_corner_is_filled_within_one_mesh_and_crisp_between_two() {
    let mut joined = Stage::top_down();
    joined.mesh(floor_and_wall(), matte(0.8), Transform::IDENTITY);
    let scene = traced(&joined);
    let radius = 0.1;
    let from = |x: f32| Vec3::new(x, 1.0, 0.2);
    let (normal, convexity, singly) = rounded_at(&scene, from(0.01 * radius), Vec3::NEG_Y, radius);
    assert!(
        (normal.x - normal.y).abs() < 0.1,
        "the floor leans to the wall: {normal}"
    );
    assert!(singly.x > 0.35 && singly.y > 0.6);
    assert!(convexity < 0.0, "an inside corner rises: {convexity}");
    let mut rng = Rng::new(3, 3);
    let hit = scene
        .closest(
            from(0.01 * radius),
            Vec3::NEG_Y,
            f32::INFINITY,
            NONE,
            &mut rng,
        )
        .expect("the floor is below");
    let rounded = scene.rounded(
        hit.triangle,
        Vec3::new(0.01 * radius, 0.0, 0.2),
        Vec3::Y,
        Vec3::Y,
        radius,
        &mut rng,
    );
    assert_eq!(rounded.convexity, 0.0, "and is not an edge to wear");
    let (normal, ..) = rounded_at(&scene, from(1.5 * radius), Vec3::NEG_Y, radius);
    assert!((normal - Vec3::Y).length() < 1.0e-5);

    let mut apart = Stage::top_down();
    apart.floor(8.0);
    apart.mesh(
        GeometryDesc::box_xyz(1.0, 2.0, 4.0),
        matte(0.8),
        Transform::at(Vec3::new(-0.5, 1.0, 0.0)),
    );
    let scene = traced(&apart);
    let (normal, ..) = rounded_at(&scene, from(0.2 * radius), Vec3::NEG_Y, radius);
    assert!(
        (normal - Vec3::Y).length() < 1.0e-5,
        "separate parts meet in a line"
    );
}

#[test]
fn the_far_side_of_a_thin_part_does_not_cancel_its_normal() {
    let mut stage = Stage::top_down();
    stage.mesh(
        GeometryDesc::box_xyz(2.0, 0.02, 2.0),
        matte(0.8),
        Transform::IDENTITY,
    );
    let scene = traced(&stage);
    let (normal, _, singly) = rounded_at(&scene, Vec3::new(0.1, 5.0, 0.2), Vec3::NEG_Y, 0.1);
    assert!((normal - Vec3::Y).length() < 1.0e-5, "{normal}");
    assert!((singly - Vec3::Y).length() < 1.0e-5);
}

#[test]
fn sheltered_places_are_found_enclosed_and_open_ones_are_not() {
    let mut stage = Stage::top_down();
    stage.mesh(floor_and_wall(), matte(0.8), Transform::IDENTITY);
    let scene = traced(&stage);
    let mut rng = Rng::new(5, 2);
    let share = |x: f32, rng: &mut Rng| {
        let from = Vec3::new(x, 1.0, 0.2);
        let hit = scene
            .closest(from, Vec3::NEG_Y, f32::INFINITY, NONE, rng)
            .expect("the floor is below");
        let position = from + Vec3::NEG_Y * hit.distance;
        let found = (0..2000)
            .filter(|_| scene.enclosed(hit.triangle, position, Vec3::Y, 0.1, rng))
            .count();
        found as f32 / 2000.0
    };
    let near = share(0.01, &mut rng);
    assert!(near > 0.3 && near < 0.6, "next to the wall: {near}");
    assert_eq!(share(0.5, &mut rng), 0.0, "out on the floor");
}

/// A glossy dark box seen edge-on, lit from where the camera stands. Neither
/// face mirrors the light back; only a rounded edge does.
fn edge_on() -> Stage {
    let eye = Vec3::new(4.0, 4.0, 0.0);
    let mut stage = Stage::new(eye, Vec3::new(1.0, 1.0, 0.0), Vec3::Y);
    let gloss =
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.01, 0.01, 0.01), 0.0, 0.2);
    stage.mesh(
        GeometryDesc::box_xyz(2.0, 2.0, 6.0),
        gloss,
        Transform::IDENTITY,
    );
    stage
        .scene
        .point_light(PointLight::default().with_intensity_candela(60.0))
        .transform(Transform::at(eye))
        .add()
        .expect("light inserts");
    stage
}

#[test]
fn a_rounded_edge_catches_the_highlight_a_sharp_one_misses() {
    let stage = edge_on();
    let renderer = renderer();
    let centre = (SIZE / 2, SIZE / 2);
    let brightness = |options: PathTraceOptions| {
        let image = stage.trace(&renderer, options);
        let pixel = radiance(&image, centre, 1);
        (
            pixel[0] + pixel[1] + pixel[2],
            image.stats().rounded_surfaces,
        )
    };
    let (sharp, none) = brightness(exact(48, 1));
    let (rounded, some) = brightness(exact(48, 1).with_edge_radius_m(0.2));
    assert_eq!((none, some), (0, 1));
    assert!(rounded > 4.0 * sharp, "rounded {rounded}, sharp {sharp}");
    let (off, _) = brightness(
        exact(48, 1)
            .with_edge_radius_m(0.2)
            .with_surface_detail(false),
    );
    assert_eq!(off, sharp, "turned off, the edge is the bare mesh again");
}

#[test]
fn a_finish_roughens_the_image_and_turning_detail_off_gives_the_plain_material() {
    let finish = SurfaceFinish::new(PhotographicSurfaceKind::CastMetal)
        .with_scale(40.0)
        .with_relief(2.0);
    let metal =
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.7, 0.7, 0.7), 1.0, 0.35);
    let stage_with = |material: MaterialDesc| {
        // Close enough that a pixel is a third of the enlarged grain.
        let mut stage = Stage::new(Vec3::new(0.0, 0.5, 0.0), Vec3::ZERO, Vec3::NEG_Z);
        stage.mesh(GeometryDesc::plane(6.0, 6.0), material, Transform::IDENTITY);
        stage
            .scene
            .directional_light(DirectionalLight::default().with_illuminance_lux(30_000.0))
            .transform(Transform::IDENTITY.looking_at(Vec3::new(1.0, -1.0, 0.3), Vec3::Y))
            .add()
            .expect("light inserts");
        stage
    };
    let renderer = renderer();
    let plain = stage_with(metal.clone()).trace(&renderer, exact(32, 1));
    let finished_stage = stage_with(metal.with_surface_finish(finish));
    let finished = finished_stage.trace(&renderer, exact(32, 1));
    let off = finished_stage.trace(&renderer, exact(32, 1).with_surface_detail(false));
    assert_eq!(
        off.rgba8(),
        plain.rgba8(),
        "detail off is the plain material"
    );
    assert_eq!(off.linear_radiance(), plain.linear_radiance());
    assert_eq!(
        (
            plain.stats().finished_surfaces,
            finished.stats().finished_surfaces
        ),
        (0, 1)
    );
    assert_eq!(off.stats().finished_surfaces, 0);

    // The plain plate shades smoothly; the cast one changes from each pixel
    // to the next, about a finite mean.
    let spread = |image: &PathTracedImage| {
        let value = |x: u32, y: u32| radiance(image, (x, y), 0)[1];
        let pixels = || (4..SIZE - 4).flat_map(|y| (4..SIZE - 4).map(move |x| (x, y)));
        let count = pixels().count() as f32;
        let mean = pixels().map(|(x, y)| value(x, y)).sum::<f32>() / count;
        let step = pixels()
            .map(|(x, y)| (value(x + 1, y) - value(x, y)).abs())
            .sum::<f32>()
            / count;
        (mean, step / mean.max(1.0e-6))
    };
    let (plain_mean, plain_spread) = spread(&plain);
    let (cast_mean, cast_spread) = spread(&finished);
    assert!(
        cast_spread > 3.0 * plain_spread + 0.05,
        "{cast_spread} against {plain_spread}"
    );
    // Away from the mirror direction a rough casting sends back far more
    // light than a smooth plate, so only gross errors are ruled out here.
    assert!(cast_mean > 0.02 * plain_mean && cast_mean < 50.0 * plain_mean);
}
