//! The CPU renderer shades triangle corners and interpolates between them. A
//! glossy surface's reflection changes faster than that across a coarse
//! triangle, so its triangles are subdivided on screen; without that a chrome
//! sphere reflected the environment as one polygon per source triangle.

use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene, Transform, Vec3,
};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 120;
const BACKGROUND: [u8; 3] = [255, 255, 255];

fn render_chrome_sphere(segments: u32, rings: u32, roughness: f32) -> Vec<u8> {
    let assets = Assets::new();
    let environment = pollster::block_on(
        assets.load_environment("tests/assets/environment/polyhaven/studio_small_08_1k.hdr"),
    )
    .expect("studio HDR loads");
    let geometry = assets.create_geometry(GeometryDesc::sphere(0.5, segments, rings));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(242, 244, 248),
        1.0,
        roughness,
    ));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("sphere inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard().with_fov_degrees(30.0),
            Transform::at(Vec3::new(0.0, 0.0, 2.4)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let mut renderer = Renderer::headless(WIDTH, HEIGHT).expect("CPU renderer builds");
    renderer.set_environment(environment);
    renderer.set_background_color(Color::WHITE);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

/// Mean absolute RGB difference, in 8-bit steps, over the sphere's interior
/// in both frames. Pixels within `EDGE_MARGIN` of the background are left out:
/// a coarse sphere's outline is a polygon, which is geometry, not reflection.
fn mean_sphere_difference(left: &[u8], right: &[u8]) -> f32 {
    const EDGE_MARGIN: i64 = 3;
    let covered = |frame: &[u8], x: i64, y: i64| {
        if x < 0 || y < 0 || x >= i64::from(WIDTH) || y >= i64::from(HEIGHT) {
            return false;
        }
        let index = ((y * i64::from(WIDTH) + x) * 4) as usize;
        frame[index..index + 3] != BACKGROUND
    };
    let interior = |frame: &[u8], x: i64, y: i64| {
        (-EDGE_MARGIN..=EDGE_MARGIN)
            .all(|dy| (-EDGE_MARGIN..=EDGE_MARGIN).all(|dx| covered(frame, x + dx, y + dy)))
    };
    let mut total = 0_u64;
    let mut count = 0_u64;
    for y in 0..i64::from(HEIGHT) {
        for x in 0..i64::from(WIDTH) {
            if !interior(left, x, y) || !interior(right, x, y) {
                continue;
            }
            let index = ((y * i64::from(WIDTH) + x) * 4) as usize;
            total += (0..3)
                .map(|c| u64::from(left[index + c].abs_diff(right[index + c])))
                .sum::<u64>();
            count += 3;
        }
    }
    assert!(count > 3 * 1_500, "the sphere must cover the frame centre");
    total as f32 / count as f32
}

#[test]
fn a_mirror_sphere_reflects_the_same_environment_at_any_tessellation() {
    // A 16x12 sphere differed from a 96x64 one by 41.4 of 255 over its
    // interior before glossy triangles were subdivided, and by 5.0 after; the
    // rest is the coarse sphere's flat facets, which a rough sphere shows too.
    let coarse = render_chrome_sphere(16, 12, 0.0);
    let fine = render_chrome_sphere(96, 64, 0.0);
    let difference = mean_sphere_difference(&coarse, &fine);
    assert!(
        difference < 8.0,
        "a coarse and a fine chrome sphere must reflect the same studio; \
         mean difference {difference:.2} of 255"
    );
}
