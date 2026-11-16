//! GPU bloom proof: a small emitter gains a round halo that falls off
//! monotonically, and a frame with nothing above the threshold is unchanged.

use crate::assets::Assets;
use crate::material::Color;
use crate::scene::{PerspectiveCamera, Scene, Transform, Vec3};

use super::super::{AntiAliasing, PostBloomConfig, Renderer, Tonemapper};
use super::{CAMERA_DISTANCE_FOR_NDC_FIXTURES, quad_primitives, render_gpu_post_frame};

const SIZE: u32 = 96;
/// The emitter covers the four pixels around the target centre: pixels 47
/// and 48 on each axis, so its centre is the corner they share.
const CENTRE: i32 = 48;

#[derive(Debug, Clone, Copy)]
struct HaloShape {
    /// Largest rise, in linear light, from one axis sample to the next one
    /// further out.
    largest_rise: f32,
    /// Smallest and largest ratio of an off-axis sample to the axis sample at
    /// the same distance from the emitter.
    roundness: (f32, f32),
    /// Halo three and seventeen pixels out from the emitter centre.
    near: f32,
    far: f32,
}

#[test]
fn gpu_bloom_gives_a_small_emitter_a_round_monotonic_halo() {
    let Some(frame) = render(40.0, Some(PostBloomConfig::new(128, 1.0, 12))) else {
        return;
    };
    let shape = measure_halo(&frame);
    eprintln!("GPU bloom halo: {shape:?}");
    evaluate_halo(shape).unwrap_or_else(|error| panic!("GPU bloom halo is malformed: {error}"));
}

#[test]
fn halo_oracle_rejects_a_plus_shaped_bloom() {
    // The nine-tap bloom this replaced: copies of the emitter on a plus.
    let mut frame = vec![0_u8; (SIZE * SIZE * 4) as usize];
    let mut stamp = |x: i32, y: i32, value: u8| {
        for (dx, dy) in [(0, 0), (-1, 0), (0, -1), (-1, -1)] {
            let offset = (((y + dy) as u32 * SIZE + (x + dx) as u32) * 4) as usize;
            frame[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    };
    for (dx, dy) in [(6, 0), (-6, 0), (0, 6), (0, -6)] {
        stamp(CENTRE + dx, CENTRE + dy, 200);
        stamp(CENTRE + dx * 2, CENTRE + dy * 2, 200);
    }
    stamp(CENTRE, CENTRE, 255);
    assert!(
        evaluate_halo(measure_halo(&frame)).is_err(),
        "a plus-shaped bloom must fail the halo oracle"
    );
}

#[test]
fn gpu_bloom_leaves_a_frame_below_the_threshold_unchanged() {
    // Linear 0.15 sits below the 128/255 threshold, which is linear 0.216.
    let active = PostBloomConfig::new(128, 1.0, 12);
    // The same post path with zero intensity adds nothing by construction.
    let inert = PostBloomConfig::new(128, 0.0, 12);
    let Some(with_bloom) = render(0.15, Some(active)) else {
        return;
    };
    let reference = render(0.15, Some(inert)).expect("inert bloom frame renders");
    assert_eq!(
        with_bloom, reference,
        "bloom must add nothing when no pixel is brighter than the threshold"
    );
    let unprocessed = render(0.15, None).expect("frame without post-processing renders");
    let largest = with_bloom
        .iter()
        .zip(&unprocessed)
        .map(|(left, right)| left.abs_diff(*right))
        .max()
        .unwrap_or(0);
    assert!(
        largest <= 1,
        "apart from output dither, bloom below the threshold must match the frame without \
         post-processing; largest channel difference {largest}"
    );

    // A bright emitter must change the frame, or the checks above would pass
    // for a bloom that never runs.
    let bright = render(40.0, Some(active)).expect("bright frame renders");
    let bright_inert = render(40.0, Some(inert)).expect("bright inert frame renders");
    assert_ne!(bright, bright_inert, "bloom must act above the threshold");
}

fn evaluate_halo(shape: HaloShape) -> Result<(), String> {
    if shape.near <= 0.02 {
        return Err(format!("no halo next to the emitter: {}", shape.near));
    }
    if shape.far >= shape.near * 0.5 {
        return Err(format!(
            "halo does not fall off: {} near against {} far",
            shape.near, shape.far
        ));
    }
    if shape.largest_rise > 0.004 {
        return Err(format!(
            "halo brightens again further out, by {} in linear light",
            shape.largest_rise
        ));
    }
    if shape.roundness.0 < 0.8 || shape.roundness.1 > 1.25 {
        return Err(format!(
            "halo is not round: off-axis samples are {} to {} of the axis sample at the same \
             distance",
            shape.roundness.0, shape.roundness.1
        ));
    }
    Ok(())
}

fn measure_halo(frame: &[u8]) -> HaloShape {
    // Offset 0 is pixel 48 and offset -1 is pixel 47, its mirror image.
    let sample = |dx: i32, dy: i32| {
        let offset = (((CENTRE + dy) as u32 * SIZE + (CENTRE + dx) as u32) * 4) as usize;
        let linear = |channel: u8| {
            let encoded = f32::from(channel) / 255.0;
            if encoded <= 0.04045 {
                encoded / 12.92
            } else {
                ((encoded + 0.055) / 1.055).powf(2.4)
            }
        };
        (linear(frame[offset]) + linear(frame[offset + 1]) + linear(frame[offset + 2])) / 3.0
    };
    // Mean over the four mirror images of one offset.
    let ring = |dx: i32, dy: i32| {
        (sample(dx, dy) + sample(-dx - 1, dy) + sample(dx, -dy - 1) + sample(-dx - 1, -dy - 1))
            / 4.0
    };
    let axis = |distance: i32| (ring(distance, 0) + ring(0, distance)) / 2.0;

    let mut largest_rise = 0.0_f32;
    for distance in 3..22 {
        largest_rise = largest_rise.max(axis(distance + 1) - axis(distance));
    }
    // Integer offsets as far from the emitter as an axis sample.
    let mut roundness = (f32::INFINITY, 0.0_f32);
    for (distance, dx, dy) in [(5, 3, 4), (10, 6, 8), (13, 5, 12), (15, 9, 12), (17, 8, 15)] {
        let on_axis = axis(distance);
        for off_axis in [ring(dx, dy), ring(dy, dx)] {
            let ratio = off_axis / on_axis.max(1.0e-6);
            roundness = (roundness.0.min(ratio), roundness.1.max(ratio));
        }
    }
    HaloShape {
        largest_rise,
        roundness,
        near: axis(3),
        far: axis(17),
    }
}

/// Renders a two-by-two pixel emitter of the given linear radiance on black.
fn render(radiance: f32, bloom: Option<PostBloomConfig>) -> Option<Vec<u8>> {
    let scene_factory = move || {
        let assets = Assets::new();
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
            )
            .expect("bloom fixture camera inserts");
        scene
            .set_active_camera(camera)
            .expect("bloom fixture camera activates");
        // One pixel is 2 / SIZE world units at this camera.
        let half = 2.0 / SIZE as f32;
        scene
            .add_renderable(
                scene.root(),
                quad_primitives(
                    -half,
                    -half,
                    half,
                    half,
                    0.0,
                    Color::from_linear_rgb(radiance, radiance, radiance),
                )
                .to_vec(),
                Transform::default(),
            )
            .expect("bloom fixture emitter inserts");
        (assets, scene, camera)
    };
    render_gpu_post_frame(SIZE, SIZE, scene_factory, |renderer: &mut Renderer| {
        renderer.set_tonemapper(Tonemapper::Standard);
        renderer.set_anti_aliasing(AntiAliasing::None);
        renderer.set_bloom(bloom);
        renderer.clear_screen_space_ambient_occlusion();
    })
    .map(|(frame, _stats)| frame)
}
