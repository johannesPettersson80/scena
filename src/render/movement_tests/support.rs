//! Shared fixtures for the retained-prepare movement tests.

use crate::assets::Assets;
use crate::scene::{CameraKey, Scene};

use super::super::{PrepareTelemetry, Renderer};

/// Which renderer a movement test drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Backend {
    Cpu,
    Gpu,
}

impl Backend {
    /// Builds a renderer, or `None` when the GPU backend has no adapter and the
    /// lane did not demand one.
    pub(super) fn renderer(self, width: u32, height: u32) -> Option<Renderer> {
        match self {
            Self::Cpu => Some(Renderer::headless(width, height).expect("CPU renderer builds")),
            Self::Gpu => super::headless_gpu_for_movement_test(width, height),
        }
    }
}

/// Prepare work recorded between two points of a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PrepareWork {
    pub(super) full_prepares: u64,
    pub(super) primitive_collections: u64,
    pub(super) static_gpu_rebuilds: u64,
    pub(super) retained_prepares: u64,
}

impl PrepareWork {
    pub(super) fn since(before: PrepareTelemetry, after: PrepareTelemetry) -> Self {
        Self {
            full_prepares: after.full_prepares - before.full_prepares,
            primitive_collections: after.prepared_primitive_collections
                - before.prepared_primitive_collections,
            static_gpu_rebuilds: after.static_gpu_resource_rebuilds
                - before.static_gpu_resource_rebuilds,
            retained_prepares: after.dynamic_template_prepares - before.dynamic_template_prepares,
        }
    }

    /// One prepare that reused the retained template and rebuilt nothing.
    pub(super) const RETAINED: Self = Self {
        full_prepares: 0,
        primitive_collections: 0,
        static_gpu_rebuilds: 0,
        retained_prepares: 1,
    };
}

/// Prepares and renders, returning the frame and the prepare work it cost.
pub(super) fn prepare_and_render(
    renderer: &mut Renderer,
    scene: &mut Scene,
    assets: &Assets,
    camera: CameraKey,
) -> (Vec<u8>, PrepareWork) {
    let before = renderer.movement_prepare_telemetry_for_test();
    renderer
        .prepare_with_assets(scene, assets)
        .expect("prepare succeeds");
    renderer.render(scene, camera).expect("render succeeds");
    let work = PrepareWork::since(before, renderer.movement_prepare_telemetry_for_test());
    (renderer.frame_rgba8().to_vec(), work)
}

/// Renders the scene as it stands through a renderer that has never seen it,
/// which can only take the full prepare path.
pub(super) fn fresh_full_prepare_frame(
    backend: Backend,
    width: u32,
    height: u32,
    scene: &mut Scene,
    assets: &Assets,
    camera: CameraKey,
) -> Vec<u8> {
    let mut renderer = backend
        .renderer(width, height)
        .expect("the reference renderer uses the backend the test already built");
    let (frame, work) = prepare_and_render(&mut renderer, scene, assets, camera);
    assert_eq!(
        work.full_prepares, 1,
        "the reference frame must come from a full prepare"
    );
    frame
}

/// One retained frame next to the frame a full prepare makes of the same scene.
pub(super) struct RetainedFrame {
    /// The frame after `change`, prepared by a renderer that already held the
    /// scene's earlier state.
    pub(super) frame: Vec<u8>,
    /// What preparing that frame cost.
    pub(super) work: PrepareWork,
    /// The same scene through a renderer that has never seen it.
    pub(super) reference: Vec<u8>,
}

impl RetainedFrame {
    /// The retained frame must be, byte for byte, the frame a full prepare
    /// produces. This holds whenever no node moved: the vertex data and every
    /// per-draw matrix are then the ones a full prepare would upload.
    pub(super) fn assert_matches_full_prepare(&self, case: &str) {
        assert_eq!(
            frame_difference(&self.frame, &self.reference),
            (0, 0),
            "{case}: (differing pixels, largest channel difference) between the retained \
             frame and a fresh full prepare must be zero"
        );
    }

    /// The retained frame must match a full prepare up to float rounding.
    ///
    /// When a node moves, the retained path keeps the model-space vertices it
    /// recovered from the first bake and draws them under the new matrix. A
    /// full prepare recovers them again through the inverse of the new matrix,
    /// so the two can disagree in the last bit of a shaded pixel. Anything
    /// larger is a real difference: a misplaced draw moves whole regions and a
    /// wrong blend order shifts channels by tens of levels.
    pub(super) fn assert_matches_full_prepare_within_rounding(&self, case: &str) {
        let (pixels, largest) = frame_difference(&self.frame, &self.reference);
        let allowed_pixels = self.frame.len() / 4 / 256;
        assert!(
            largest <= 2 && pixels <= allowed_pixels,
            "{case}: the retained frame may differ from a fresh full prepare only by float \
             rounding (at most {allowed_pixels} pixels by at most 2 levels), but {pixels} pixels \
             differ by up to {largest}"
        );
    }
}

/// Prepares and renders `scene`, applies `change`, and prepares and renders
/// again on the same renderer. `None` when the backend has no adapter.
pub(super) fn retained_frame_after(
    backend: Backend,
    size: u32,
    scene: &mut Scene,
    assets: &Assets,
    change: impl FnOnce(&mut Scene),
) -> Option<RetainedFrame> {
    let camera = scene
        .active_camera()
        .expect("the scene has an active camera");
    let mut renderer = backend.renderer(size, size)?;
    prepare_and_render(&mut renderer, scene, assets, camera);
    change(scene);
    let camera = scene
        .active_camera()
        .expect("the scene has an active camera");
    let (frame, work) = prepare_and_render(&mut renderer, scene, assets, camera);
    let reference = fresh_full_prepare_frame(backend, size, size, scene, assets, camera);
    Some(RetainedFrame {
        frame,
        work,
        reference,
    })
}

/// Number of pixels whose RGBA differs between two frames, and the largest
/// single-channel difference.
pub(super) fn frame_difference(left: &[u8], right: &[u8]) -> (usize, u8) {
    assert_eq!(left.len(), right.len(), "frames must share a size");
    let mut pixels = 0;
    let mut largest = 0;
    for (left, right) in left.chunks_exact(4).zip(right.chunks_exact(4)) {
        let delta = left
            .iter()
            .zip(right)
            .map(|(left, right)| left.abs_diff(*right))
            .max()
            .unwrap_or(0);
        if delta > 0 {
            pixels += 1;
            largest = largest.max(delta);
        }
    }
    (pixels, largest)
}

/// Centroid of the pixels that are not the black clear colour.
pub(super) fn visible_centroid(frame: &[u8], width: u32) -> Option<(f32, f32)> {
    let mut sum_x = 0.0_f32;
    let mut sum_y = 0.0_f32;
    let mut count = 0_u32;
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        if pixel[..3].iter().any(|channel| *channel > 16) {
            sum_x += (index as u32 % width) as f32;
            sum_y += (index as u32 / width) as f32;
            count += 1;
        }
    }
    (count > 0).then(|| (sum_x / count as f32, sum_y / count as f32))
}
