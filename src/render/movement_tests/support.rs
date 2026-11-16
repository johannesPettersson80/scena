//! Shared fixtures for the retained-prepare movement tests.

use crate::assets::Assets;
use crate::scene::{CameraKey, Scene, Transform, Vec3};

use super::super::prepare_lifecycle::FullPrepareReason;
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
    /// Why that prepare baked the scene again, if it did.
    pub(super) full_prepare_reason: Option<FullPrepareReason>,
    /// The same scene through a renderer that has never seen it.
    pub(super) reference: Vec<u8>,
    /// [`DrawnStats`] of the renderer under test and of the reference.
    pub(super) stats: (DrawnStats, DrawnStats),
}

/// What a renderer reports about the frame it just drew, for the counts a
/// retained prepare has to keep in step with a full one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DrawnStats {
    triangles: u64,
    instances: u64,
    culled_objects: u64,
    depth_prepass_passes: u64,
    depth_prepass_draws: u64,
}

impl DrawnStats {
    pub(super) fn of(renderer: &Renderer) -> Self {
        let stats = renderer.stats();
        Self {
            triangles: stats.triangles,
            instances: stats.instances,
            culled_objects: stats.culled_objects,
            depth_prepass_passes: stats.depth_prepass_passes,
            depth_prepass_draws: stats.depth_prepass_draws,
        }
    }
}

impl RetainedFrame {
    /// The change was absorbed by the retained template, and the frame is
    /// byte for byte the one a full prepare produces.
    pub(super) fn assert_retained(&self, case: &str) {
        assert_eq!(
            (self.work, self.full_prepare_reason),
            (PrepareWork::RETAINED, None),
            "{case}: the prepare must reuse the retained template"
        );
        self.assert_matches_full_prepare(case);
    }

    /// The change made the prepare bake the scene again, for `reason` and
    /// no other, and the frame is the one a renderer new to the scene
    /// produces.
    pub(super) fn assert_full_prepare(&self, reason: FullPrepareReason, case: &str) {
        assert_eq!(
            (
                self.work.full_prepares,
                self.work.retained_prepares,
                self.full_prepare_reason,
            ),
            (1, 0, Some(reason)),
            "{case}: (full prepares, retained prepares, reason for the full prepare)"
        );
        self.assert_matches_full_prepare(case);
    }

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
        self.assert_reports_what_a_full_prepare_reports(case);
    }

    /// The renderer must report the frame it drew as a renderer new to the
    /// scene reports it.
    fn assert_reports_what_a_full_prepare_reports(&self, case: &str) {
        assert_eq!(
            self.stats.0, self.stats.1,
            "{case}: renderer stats after the retained prepare and after a fresh full prepare"
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
        self.assert_reports_what_a_full_prepare_reports(case);
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
    retained_frame_after_configured(backend, size, scene, assets, |_renderer| {}, change)
}

/// [`retained_frame_after`] with `configure` applied to the renderer under
/// test and to the reference renderer before either sees the scene.
pub(super) fn retained_frame_after_configured(
    backend: Backend,
    size: u32,
    scene: &mut Scene,
    assets: &Assets,
    configure: impl Fn(&mut Renderer),
    change: impl FnOnce(&mut Scene),
) -> Option<RetainedFrame> {
    let camera = scene
        .active_camera()
        .expect("the scene has an active camera");
    let mut renderer = backend.renderer(size, size)?;
    configure(&mut renderer);
    prepare_and_render(&mut renderer, scene, assets, camera);
    change(scene);
    let camera = scene
        .active_camera()
        .expect("the scene has an active camera");
    let (frame, work) = prepare_and_render(&mut renderer, scene, assets, camera);
    let full_prepare_reason = (work.full_prepares > 0)
        .then(|| {
            renderer
                .movement_prepare_telemetry_for_test()
                .last_full_prepare_reason
        })
        .flatten();
    let mut fresh = backend
        .renderer(size, size)
        .expect("the reference renderer uses the backend the test already built");
    configure(&mut fresh);
    let (reference, fresh_work) = prepare_and_render(&mut fresh, scene, assets, camera);
    assert_eq!(
        fresh_work.full_prepares, 1,
        "the reference frame must come from a full prepare"
    );
    Some(RetainedFrame {
        frame,
        work,
        full_prepare_reason,
        reference,
        stats: (DrawnStats::of(&renderer), DrawnStats::of(&fresh)),
    })
}

/// Places the camera on a horizontal circle around the origin, looking at it.
pub(super) fn orbit_camera(scene: &mut Scene, camera: CameraKey, yaw_degrees: f32, radius: f32) {
    let yaw = yaw_degrees.to_radians();
    let eye = Vec3::new(radius * yaw.sin(), 0.0, radius * yaw.cos());
    let node = scene.camera_node(camera).expect("camera node resolves");
    scene
        .set_transform(node, Transform::at(eye).looking_at(Vec3::ZERO, Vec3::Y))
        .expect("camera orbits");
}

pub(super) fn pixel(frame: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let index = ((y * width + x) * 4) as usize;
    [
        frame[index],
        frame[index + 1],
        frame[index + 2],
        frame[index + 3],
    ]
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
