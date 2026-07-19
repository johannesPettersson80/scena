#![cfg(all(not(target_arch = "wasm32"), feature = "inspection"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    Assets, Callout, MeasurementOverlay, RenderQualityRegion, RenderQualityStatusV1, Renderer,
    Scene, SceneRecipeQualityLineV1, SceneRecipeQualityTextV1, UnitFormat, Vec3,
    evaluate_label_region_quality, evaluate_line_region_quality, screen_region_from_center_size,
    screen_region_from_points,
};

#[path = "support/q03_visual_metrics.rs"]
mod q03_visual_metrics;
use q03_visual_metrics::{PixelRect, clear_rect};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 120;

#[test]
fn measurement_and_callout_independently_prove_line_and_label_quality() {
    let proofs = [render_measurement_overlay(), render_callout_overlay()];
    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/measurement");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");

    for proof in proofs {
        assert_overlay_quality(&proof);
        let mut deleted_line = proof.frame.clone();
        clear_rect(
            &mut deleted_line,
            proof.width,
            proof.height,
            pixel_rect(proof.line_region),
        );
        assert!(
            deleted_line
                .chunks_exact(4)
                .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
            "the old nonblack oracle would accept {} with its line deleted",
            proof.name,
        );
        let deleted_checks = evaluate_line_region_quality(
            "deleted-line-mutation",
            &deleted_line,
            proof.width,
            proof.height,
            proof.line_region,
            line_expectation(),
        );
        assert!(
            deleted_checks.iter().any(|check| {
                check.code == "line_missing_antialiasing"
                    && check.status == RenderQualityStatusV1::Failed
            }),
            "deleting the {} line must fail the line-specific evaluator: {deleted_checks:#?}",
            proof.name,
        );
        write_ppm_artifact(
            &artifact_dir,
            proof.name,
            proof.width,
            proof.height,
            &proof.frame,
        );
        write_ppm_artifact(
            &artifact_dir,
            &format!("{}-known-bad-deleted-line", proof.name),
            proof.width,
            proof.height,
            &deleted_line,
        );
    }
}

fn render_measurement_overlay() -> OverlayProof {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let start = Vec3::new(-0.6, -0.25, 0.0);
    let end = Vec3::new(0.6, -0.25, 0.0);
    let label_position = Vec3::new(0.0, -0.01, 0.0);
    let report = scene
        .add_measurement_overlay(
            &assets,
            MeasurementOverlay::distance("width", start, end)
                .with_label("width")
                .with_units(UnitFormat::meters().with_precision(2)),
        )
        .expect("measurement overlay inserts");
    let label = report.label.expect("visual proof includes label geometry");
    let line_region = projected_line_region(&scene, camera, start, end);
    let label_region = projected_label_region(&scene, camera, label, label_position);
    render_overlay(
        "measurement-distance-line-label",
        scene,
        camera,
        &assets,
        line_region,
        label_region,
    )
}

fn render_callout_overlay() -> OverlayProof {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let anchor = Vec3::new(-0.55, -0.25, 0.0);
    let label_offset = Vec3::new(0.35, 0.32, 0.0);
    let label_position = anchor + label_offset;
    let report = scene
        .add_callout(
            &assets,
            Callout::world("motor", anchor, "Motor").with_label_offset(label_offset),
        )
        .expect("callout overlay inserts");
    let line_region = projected_line_region(&scene, camera, anchor, label_position);
    let label_region = projected_label_region(&scene, camera, report.label, label_position);
    render_overlay(
        "callout-leader-line-label",
        scene,
        camera,
        &assets,
        line_region,
        label_region,
    )
}

fn render_overlay(
    name: &'static str,
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets,
    line_region: RenderQualityRegion,
    label_region: RenderQualityRegion,
) -> OverlayProof {
    let mut renderer = Renderer::headless(WIDTH, HEIGHT).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("overlay scene prepares");
    renderer
        .render(&scene, camera)
        .expect("overlay scene renders");
    OverlayProof {
        name,
        width: WIDTH,
        height: HEIGHT,
        frame: renderer.frame_rgba8().to_vec(),
        line_region,
        label_region,
    }
}

fn projected_line_region(
    scene: &Scene,
    camera: scena::CameraKey,
    start: Vec3,
    end: Vec3,
) -> RenderQualityRegion {
    let start = scene
        .project_world_point(camera, start, WIDTH, HEIGHT)
        .expect("line start projects")
        .expect("line start is visible");
    let end = scene
        .project_world_point(camera, end, WIDTH, HEIGHT)
        .expect("line end projects")
        .expect("line end is visible");
    let region =
        screen_region_from_points(&[(start.x, start.y), (end.x, end.y)], 3.0, WIDTH, HEIGHT)
            .expect("line region is bounded");
    RenderQualityRegion {
        kind: "line",
        handle: None,
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

fn projected_label_region(
    scene: &Scene,
    camera: scena::CameraKey,
    label: scena::LabelKey,
    position: Vec3,
) -> RenderQualityRegion {
    let projected = scene
        .project_world_point(camera, position, WIDTH, HEIGHT)
        .expect("label position projects")
        .expect("label position is visible");
    let metrics = scene.label(label).expect("label resolves").metrics();
    let region = screen_region_from_center_size(
        projected.x,
        projected.y,
        metrics.width_px,
        metrics.height_px,
        3.0,
        WIDTH,
        HEIGHT,
    )
    .expect("label region is bounded");
    RenderQualityRegion {
        kind: "label",
        handle: None,
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

fn assert_overlay_quality(proof: &OverlayProof) {
    let line_checks = evaluate_line_region_quality(
        proof.name,
        &proof.frame,
        proof.width,
        proof.height,
        proof.line_region,
        line_expectation(),
    );
    assert!(
        line_checks
            .iter()
            .all(|check| check.status != RenderQualityStatusV1::Failed),
        "{} line region must pass the existing line evaluator: {line_checks:#?}",
        proof.name,
    );
    let label_checks = evaluate_label_region_quality(
        proof.name,
        &proof.frame,
        proof.width,
        proof.height,
        proof.label_region,
        label_expectation(),
    );
    assert!(
        label_checks
            .iter()
            .all(|check| check.status != RenderQualityStatusV1::Failed),
        "{} label region must pass the existing label evaluator: {label_checks:#?}",
        proof.name,
    );
}

const fn line_expectation() -> SceneRecipeQualityLineV1 {
    SceneRecipeQualityLineV1 {
        min_intermediate_edge_fraction: Some(0.01),
        max_straightness_error: Some(0.20),
    }
}

const fn label_expectation() -> SceneRecipeQualityTextV1 {
    SceneRecipeQualityTextV1 {
        min_ink_coverage: Some(0.05),
        max_ink_isolation: Some(0.10),
        min_intermediate_edge_fraction: Some(0.005),
        max_background_luminance_range: None,
        max_background_mean_delta: None,
    }
}

fn pixel_rect(region: RenderQualityRegion) -> PixelRect {
    PixelRect {
        min_x: region.x,
        min_y: region.y,
        max_x: region.x + region.width.saturating_sub(1),
        max_y: region.y + region.height.saturating_sub(1),
    }
}

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact writes");
}

struct OverlayProof {
    name: &'static str,
    width: u32,
    height: u32,
    frame: Vec<u8>,
    line_region: RenderQualityRegion,
    label_region: RenderQualityRegion,
}
