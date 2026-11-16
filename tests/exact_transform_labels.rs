//! Rendered proof that a label is anchored where the exact world matrix puts
//! its node origin, on the CPU rasterizer and, when a headless adapter is
//! available, on the GPU path.
//!
//! A screen-aligned label has no world orientation, so the whole contract is
//! its anchor: under a rotated, non-uniformly scaled parent the label must
//! render exactly like a top-level twin placed at the matrix-computed point.
#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use scena::{
    Assets, CameraKey, Color, LabelDesc, Mat4, OrthographicCamera, Renderer, Scene, Transform, Vec3,
};

const SIZE: u32 = 200;
/// World units visible on each side of the origin.
const HALF_EXTENT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Cpu,
    Gpu,
}

/// Three levels: the label node is offset and rotated under a rotated node
/// under a rotated, non-uniformly scaled root, so its world matrix is sheared.
fn sheared_chain() -> [Transform; 3] {
    [
        Transform::at(Vec3::new(-0.4, 0.3, 0.0))
            .rotate_z_deg(30.0)
            .with_scale(Vec3::new(2.4, 0.8, 1.0)),
        Transform::at(Vec3::new(0.35, 0.2, 0.0)).rotate_z_deg(40.0),
        Transform::at(Vec3::new(0.3, 0.45, 0.1)).rotate_x_deg(25.0),
    ]
}

#[test]
fn cpu_label_under_a_sheared_parent_is_anchored_by_the_world_matrix() {
    let renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    assert_label_is_anchored_by_the_world_matrix(Backend::Cpu, renderer);
}

#[test]
fn gpu_label_under_a_sheared_parent_is_anchored_by_the_world_matrix() {
    let Some(renderer) = headless_gpu_renderer("gpu_label_under_a_sheared_parent") else {
        return;
    };
    assert_label_is_anchored_by_the_world_matrix(Backend::Gpu, renderer);
}

fn assert_label_is_anchored_by_the_world_matrix(backend: Backend, mut renderer: Renderer) {
    let chain = sheared_chain();
    let anchor = ground_truth(&chain).transform_point3(Vec3::ZERO);
    // Where composing translation, rotation, and scale component by component
    // would have put the node origin.
    let approximate = chain[1..]
        .iter()
        .fold(chain[0], |world, local| Transform::compose(world, *local))
        .translation;
    assert!(
        to_pixels(approximate).distance(to_pixels(anchor)) > 6.0,
        "the fixture must move the anchor visibly: exact={anchor:?}, approximate={approximate:?}"
    );

    let parented = render_label(&mut renderer, |scene| {
        let mut parent = scene.root();
        for local in &chain[..2] {
            parent = scene.add_empty(parent, *local).expect("ancestor inserts");
        }
        (parent, chain[2])
    });
    let twin = render_label(&mut renderer, |scene| (scene.root(), Transform::at(anchor)));

    let centre = label_centre(&twin)
        .unwrap_or_else(|| panic!("{backend:?}: the twin label must render visible pixels"));
    let expected = to_pixels(anchor);
    assert!(
        (centre.0 - expected.x).abs() <= 1.5 && (centre.1 - expected.y).abs() <= 1.5,
        "{backend:?}: the twin label is centred at {centre:?} but its anchor projects to {expected:?}"
    );
    // The two anchors agree to the last bits of a matrix product, which can
    // still tip the coverage of a glyph edge pixel on a GPU rasterizer.
    let channel_differences = parented
        .iter()
        .zip(&twin)
        .map(|(left, right)| left.abs_diff(*right));
    let differing = channel_differences
        .clone()
        .filter(|delta| *delta > 0)
        .count();
    let largest = channel_differences.max().unwrap_or(0);
    assert!(
        differing <= 16 && largest <= 8,
        "{backend:?}: the parented label must render like its matrix-placed twin; \
         {differing} channel values differ, by up to {largest}"
    );
}

fn render_label(
    renderer: &mut Renderer,
    place: impl FnOnce(&mut Scene) -> (scena::NodeKey, Transform),
) -> Vec<u8> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = add_orthographic_camera(&mut scene);
    let (parent, local) = place(&mut scene);
    scene
        .add_label(
            parent,
            LabelDesc::new("AXIS")
                .with_size(28.0)
                .with_color(Color::WHITE),
            local,
        )
        .expect("label inserts");
    renderer.set_background_color(Color::BLACK);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

/// Centre of the pixel bounding box of everything brighter than the background.
fn label_centre(frame: &[u8]) -> Option<(f32, f32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        if u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2]) < 60 {
            continue;
        }
        let x = index as u32 % SIZE;
        let y = index as u32 / SIZE;
        bounds = Some(match bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    }
    bounds.map(|(min_x, min_y, max_x, max_y)| {
        (
            (min_x + max_x) as f32 * 0.5 + 0.5,
            (min_y + max_y) as f32 * 0.5 + 0.5,
        )
    })
}

/// Pixel position of a world point under the orthographic test camera, which
/// looks down -Z with +Y up.
fn to_pixels(world: Vec3) -> glam::Vec2 {
    let pixels_per_unit = SIZE as f32 / (2.0 * HALF_EXTENT);
    glam::Vec2::new(
        (world.x + HALF_EXTENT) * pixels_per_unit,
        (HALF_EXTENT - world.y) * pixels_per_unit,
    )
}

fn add_orthographic_camera(scene: &mut Scene) -> CameraKey {
    let camera = scene
        .add_orthographic_camera(
            scene.root(),
            OrthographicCamera {
                left: -HALF_EXTENT,
                right: HALF_EXTENT,
                bottom: -HALF_EXTENT,
                top: HALF_EXTENT,
                near: 0.1,
                far: 20.0,
            },
            Transform::at(Vec3::new(0.0, 0.0, 10.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    camera
}

fn headless_gpu_renderer(test_name: &str) -> Option<Renderer> {
    support::parity::configure_lavapipe_adapter();
    match Renderer::headless_gpu(SIZE, SIZE) {
        Ok(renderer) => Some(renderer),
        Err(error) => {
            eprintln!("skipping {test_name}: no headless GPU adapter is available ({error:?})");
            None
        }
    }
}

fn matrix_of(local: Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}

fn ground_truth(chain: &[Transform]) -> Mat4 {
    chain
        .iter()
        .fold(Mat4::IDENTITY, |world, local| world * matrix_of(*local))
}
