#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use scena::{Color, LabelDesc, PerspectiveCamera, Renderer, Scene, Transform, Vec3};

#[cfg(feature = "inspection")]
use scena::{RenderQualityRegion, SceneRecipeQualityTextV1, evaluate_label_region_quality};

#[cfg(feature = "scene-host")]
fn configure_headless_gpu_test_adapter() {
    const LAVAPIPE_ICD: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";
    if std::env::var_os("VK_ICD_FILENAMES").is_none() && Path::new(LAVAPIPE_ICD).exists() {
        // SAFETY: these GPU tests set the process adapter hint immediately before
        // constructing wgpu and do not read it concurrently themselves. This keeps
        // the default cargo-test lane able to exercise the lavapipe proof on hosts
        // where the ICD is installed but not selected by the shell.
        unsafe {
            std::env::set_var("VK_ICD_FILENAMES", LAVAPIPE_ICD);
        }
    }
}

#[test]
fn label_desc_reports_stable_font_metrics_and_style_options() {
    let label = LabelDesc::new("Hi")
        .with_size(14.0)
        .with_background(Color::from_srgb_u8(8, 12, 20))
        .with_halo(Color::from_srgb_u8(220, 240, 255));

    let metrics = label.metrics();
    assert_eq!(metrics.glyph_count, 2);
    assert!(
        metrics.width_px > 10.0 && metrics.width_px < 22.0,
        "default embedded TrueType metrics should be proportional, not fixed bitmap cells: {metrics:?}"
    );
    assert!(
        metrics.height_px >= 14.0 && metrics.baseline_px > 8.0,
        "default embedded TrueType metrics should expose usable line metrics: {metrics:?}"
    );
    assert_eq!(label.background(), Some(Color::from_srgb_u8(8, 12, 20)));
    assert_eq!(label.halo(), Some(Color::from_srgb_u8(220, 240, 255)));
}

#[test]
fn label_desc_translucent_explicit_colors_fail_closed_until_transparent_path_exists() {
    for (name, label) in [
        (
            "text color",
            LabelDesc::new("A").with_color(Color::from_linear_rgba(1.0, 1.0, 1.0, 0.5)),
        ),
        (
            "background",
            LabelDesc::new("A").with_background(Color::from_linear_rgba(0.0, 0.0, 0.0, 0.5)),
        ),
        (
            "halo",
            LabelDesc::new("A").with_halo(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)),
        ),
    ] {
        let mut scene = Scene::new();
        let error = scene
            .add_label(scene.root(), label, Transform::IDENTITY)
            .expect_err(
                "translucent label colors must not be accepted without a transparent GPU path",
            );
        assert!(
            error.to_string().contains("opaque"),
            "{name} should fail closed with an opaque-label explanation: {error}"
        );
    }
}

#[test]
fn label_desc_default_font_is_embedded_truetype_and_explicit_fonts_change_metrics() {
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let default_label = LabelDesc::new("AVAV")
        .with_size(28.0)
        .with_color(Color::RED);
    let explicit = LabelDesc::truetype("AVAV", font_bytes)
        .expect("TrueType font loads")
        .with_size(28.0)
        .with_color(Color::GREEN);

    let default_metrics = default_label.metrics();
    let explicit_metrics = explicit.metrics();
    assert_eq!(default_metrics.glyph_count, 4);
    assert_eq!(explicit_metrics.glyph_count, 4);
    assert!(
        (default_metrics.width_px - explicit_metrics.width_px).abs() >= 1.0,
        "caller-supplied TrueType metrics must not silently use the embedded default face: default={default_metrics:?} explicit={explicit_metrics:?}"
    );

    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 4.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_label(
            scene.root(),
            default_label,
            Transform::at(Vec3::new(-0.45, 0.0, 0.0)),
        )
        .expect("default label inserts");
    scene
        .add_label(
            scene.root(),
            explicit,
            Transform::at(Vec3::new(0.45, 0.0, 0.0)),
        )
        .expect("explicit truetype label inserts");

    let mut renderer = Renderer::headless(220, 120).expect("renderer builds");
    renderer.prepare(&mut scene).expect("font scene prepares");
    renderer.render_active(&scene).expect("font scene renders");
    let frame = renderer.frame_rgba8().to_vec();

    let default_bounds = color_bounds(&frame, 220, 120, |pixel| {
        pixel[0] > 90
            && pixel[0] > pixel[1].saturating_mul(2)
            && pixel[0] > pixel[2].saturating_mul(2)
    })
    .expect("default label renders");
    let explicit_bounds = color_bounds(&frame, 220, 120, |pixel| {
        pixel[1] > 80
            && pixel[1] > pixel[0].saturating_mul(2)
            && pixel[1] > pixel[2].saturating_mul(2)
    })
    .expect("explicit truetype label renders");
    assert!(
        (default_bounds.width() as i32 - explicit_bounds.width() as i32).abs() >= 1
            || (default_bounds.height() as i32 - explicit_bounds.height() as i32).abs() >= 1
            || (default_bounds.occupancy() - explicit_bounds.occupancy()).abs() >= 0.02,
        "caller-supplied TrueType coverage must be distinguishable from the embedded default face: default={default_bounds:?} explicit={explicit_bounds:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "default-vs-explicit-truetype-label-text",
        220,
        120,
        &frame,
    );
}

#[test]
fn label_text_truetype_preserves_antialiasing_coverage_on_cpu() {
    let (width, height, frame) = render_truetype_aa_label(false);
    let intermediate = intermediate_gray_pixel_count(&frame);
    assert!(
        intermediate >= 32,
        "TrueType labels must preserve native glyph antialiasing coverage on CPU; expected intermediate edge pixels, found {intermediate}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "truetype-label-aa-cpu",
        width,
        height,
        &frame,
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn label_text_truetype_preserves_antialiasing_coverage_on_headless_gpu() {
    let (width, height, frame) = render_truetype_aa_label(true);
    let intermediate = intermediate_gray_pixel_count(&frame);
    assert!(
        intermediate >= 32,
        "TrueType labels must preserve native glyph antialiasing coverage on HeadlessGpu; expected intermediate edge pixels, found {intermediate}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "truetype-label-aa-gpu",
        width,
        height,
        &frame,
    );
}

#[test]
fn label_desc_truetype_rejects_complex_script_text() {
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let error = LabelDesc::truetype("سلام", font_bytes).expect_err("complex script rejected");
    assert!(
        error.to_string().contains("basic Latin"),
        "error should tell an agent the supported font scope: {error}"
    );
}

#[test]
fn label_text_renders_atlas_glyphs_with_pixel_stable_billboards() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 4.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_label(
            scene.root(),
            LabelDesc::new("HI")
                .with_size(14.0)
                .with_color(Color::RED)
                .with_halo(Color::from_srgb_u8(255, 220, 220)),
            Transform::at(Vec3::new(-0.45, 0.2, 0.0)),
        )
        .expect("near label inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::new("HI")
                .with_size(14.0)
                .with_color(Color::GREEN)
                .with_background(Color::from_srgb_u8(5, 14, 8)),
            Transform::at(Vec3::new(0.45, -0.2, -1.0)),
        )
        .expect("far label inserts");

    let mut renderer = Renderer::headless(160, 120).expect("renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("label text scene prepares");
    renderer.render_active(&scene).expect("label text renders");
    let frame = renderer.frame_rgba8().to_vec();

    let red = color_bounds(&frame, 160, 120, |pixel| {
        pixel[0] > 90
            && pixel[0] > pixel[1].saturating_mul(2)
            && pixel[0] > pixel[2].saturating_mul(2)
    })
    .expect("red near label renders");
    let green = color_bounds(&frame, 160, 120, |pixel| {
        pixel[1] > 80
            && pixel[1] > pixel[0].saturating_mul(2)
            && pixel[1] > pixel[2].saturating_mul(2)
    })
    .expect("green far label renders");

    assert!(
        (red.width() as i32 - green.width() as i32).abs() <= 3,
        "screen-aligned labels should keep roughly constant pixel width across depth: red={red:?} green={green:?}"
    );
    assert!(
        (red.height() as i32 - green.height() as i32).abs() <= 3,
        "screen-aligned labels should keep roughly constant pixel height across depth: red={red:?} green={green:?}"
    );
    assert!(
        red.occupancy() < 0.75 && green.occupancy() < 0.75,
        "labels must render glyph-shaped coverage, not a filled rectangle: red={red:?} green={green:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "default-truetype-label-text",
        160,
        120,
        &frame,
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn label_text_truetype_atlas_matches_cpu_and_headless_gpu_with_tolerance() {
    let (width, height, cpu_frame) = render_truetype_aa_label(false);
    let (_, _, gpu_frame) = render_truetype_aa_label(true);
    let crop = truetype_aa_label_region(width, height);
    let background_band = truetype_aa_label_background_band(width, height);
    let diff = frame_delta_in_bounds(&cpu_frame, &gpu_frame, width, crop);
    let background_diff = frame_delta_in_bounds(&cpu_frame, &gpu_frame, width, background_band);
    let cpu_background_luma = mean_luminance_in_bounds(&cpu_frame, width, background_band);
    let gpu_background_luma = mean_luminance_in_bounds(&gpu_frame, width, background_band);
    let cpu_intermediate = intermediate_gray_pixel_count(&cpu_frame);
    let gpu_intermediate = intermediate_gray_pixel_count(&gpu_frame);
    assert!(
        cpu_intermediate >= 32 && gpu_intermediate >= 32,
        "both backends must preserve native glyph AA coverage, cpu={cpu_intermediate} gpu={gpu_intermediate}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "truetype-label-atlas-parity-cpu",
        width,
        height,
        &cpu_frame,
    );
    write_ppm_artifact(
        &artifact_dir,
        "truetype-label-atlas-parity-gpu",
        width,
        height,
        &gpu_frame,
    );
    assert_live_atlas_quality_passes(width, height, &cpu_frame, &gpu_frame, crop);
    write_ppm_crop_artifact(
        &artifact_dir,
        "truetype-label-atlas-parity-cpu-crop",
        width,
        &cpu_frame,
        crop,
    );
    write_ppm_crop_artifact(
        &artifact_dir,
        "truetype-label-atlas-parity-gpu-crop",
        width,
        &gpu_frame,
        crop,
    );
    write_ppm_crop_artifact(
        &artifact_dir,
        "truetype-label-atlas-parity-cpu-background-band",
        width,
        &cpu_frame,
        background_band,
    );
    write_ppm_crop_artifact(
        &artifact_dir,
        "truetype-label-atlas-parity-gpu-background-band",
        width,
        &gpu_frame,
        background_band,
    );
    fs::write(
        artifact_dir.join("truetype-label-atlas-parity.json"),
        format!(
            "{{\n  \"schema\": \"scena.label_atlas_parity.v1\",\n  \"max_channel_delta\": {},\n  \"mean_channel_delta\": {:.3},\n  \"background_max_channel_delta\": {},\n  \"background_mean_channel_delta\": {:.3},\n  \"cpu_background_luma\": {:.3},\n  \"gpu_background_luma\": {:.3},\n  \"cpu_intermediate_gray_pixels\": {},\n  \"gpu_intermediate_gray_pixels\": {}\n}}\n",
            diff.max_channel_delta,
            diff.mean_channel_delta,
            background_diff.max_channel_delta,
            background_diff.mean_channel_delta,
            cpu_background_luma,
            gpu_background_luma,
            cpu_intermediate,
            gpu_intermediate
        ),
    )
    .expect("parity artifact writes");
    assert!(
        diff.max_channel_delta <= 220 && diff.mean_channel_delta <= 6.0,
        "CPU and HeadlessGpu label atlas renders must match within tolerance over the full label region, diff={diff:?}, crop={crop:?}"
    );
    assert!(
        background_diff.max_channel_delta <= 24 && background_diff.mean_channel_delta <= 6.0,
        "CPU and HeadlessGpu label background pill must match within tolerance over the background band, diff={background_diff:?}, cpu_luma={cpu_background_luma:.3}, gpu_luma={gpu_background_luma:.3}, band={background_band:?}"
    );
}

#[cfg(feature = "scene-host")]
fn truetype_aa_label_region(width: u32, height: u32) -> PixelBounds {
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let label = LabelDesc::truetype("BATTERY", font_bytes)
        .expect("TrueType label builds")
        .with_size(42.0)
        .with_color(Color::WHITE)
        .with_background(Color::DARK_GRAY);
    let metrics = label.metrics();
    let padding = (label.size() * 0.25).ceil().max(2.0);
    let label_width = metrics.width_px + padding * 2.0;
    let label_height = metrics.height_px + padding * 2.0;
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;
    PixelBounds {
        min_x: (center_x - label_width * 0.5).floor().max(0.0) as u32,
        min_y: (center_y - label_height * 0.5).floor().max(0.0) as u32,
        max_x: (center_x + label_width * 0.5)
            .ceil()
            .min(width.saturating_sub(1) as f32) as u32,
        max_y: (center_y + label_height * 0.5)
            .ceil()
            .min(height.saturating_sub(1) as f32) as u32,
        pixels: 0,
    }
}

#[cfg(feature = "scene-host")]
fn truetype_aa_label_background_band(width: u32, height: u32) -> PixelBounds {
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let label = LabelDesc::truetype("BATTERY", font_bytes)
        .expect("TrueType label builds")
        .with_size(42.0)
        .with_color(Color::WHITE)
        .with_background(Color::DARK_GRAY);
    let metrics = label.metrics();
    let padding = (label.size() * 0.25).ceil().max(2.0);
    let label_width = metrics.width_px + padding * 2.0;
    let label_height = metrics.height_px + padding * 2.0;
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;
    let left = (center_x - label_width * 0.5).floor().max(0.0);
    let top = (center_y - label_height * 0.5).floor().max(0.0);
    let right = (center_x + label_width * 0.5)
        .ceil()
        .min(width.saturating_sub(1) as f32);
    let y0 = top + 2.0;
    let y1 = (top + padding - 3.0).max(y0);
    PixelBounds {
        min_x: (left + 2.0).min(right) as u32,
        min_y: y0.min(height.saturating_sub(1) as f32) as u32,
        max_x: (right - 2.0).max(left).min(width.saturating_sub(1) as f32) as u32,
        max_y: y1.min(height.saturating_sub(1) as f32) as u32,
        pixels: 0,
    }
}

#[cfg(all(feature = "scene-host", feature = "inspection"))]
fn assert_live_atlas_quality_passes(
    width: u32,
    height: u32,
    cpu_frame: &[u8],
    gpu_frame: &[u8],
    crop: PixelBounds,
) {
    let quality_region = RenderQualityRegion {
        kind: "label",
        handle: Some(1),
        x: crop.min_x,
        y: crop.min_y,
        width: crop.width(),
        height: crop.height(),
    };
    let quality_expectation = SceneRecipeQualityTextV1 {
        min_ink_coverage: Some(0.06),
        max_ink_isolation: Some(0.02),
        min_intermediate_edge_fraction: Some(0.01),
        max_background_luminance_range: None,
        max_background_mean_delta: None,
    };
    for (backend, frame) in [("CPU", cpu_frame), ("HeadlessGpu", gpu_frame)] {
        let checks = evaluate_label_region_quality(
            backend,
            frame,
            width,
            height,
            quality_region,
            quality_expectation,
        );
        assert!(
            checks.iter().all(|check| check.severity != "error"),
            "live {backend} atlas label render must pass the same per-label quality verifier used by recipes: {checks:#?}"
        );
    }
}

#[cfg(all(feature = "scene-host", not(feature = "inspection")))]
fn assert_live_atlas_quality_passes(
    _width: u32,
    _height: u32,
    _cpu_frame: &[u8],
    _gpu_frame: &[u8],
    _crop: PixelBounds,
) {
}

#[cfg(feature = "scene-host")]
#[test]
fn label_text_headless_gpu_proves_color_position_size_and_depth() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 2.2)).looking_at(Vec3::ZERO, Vec3::Y),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_label(
            scene.root(),
            LabelDesc::new("BIG")
                .with_size(30.0)
                .with_color(Color::from_srgb_u8(32, 80, 230))
                .with_background(Color::from_srgb_u8(32, 80, 230)),
            Transform::at(Vec3::new(0.0, 0.0, -0.35)),
        )
        .expect("far blue label inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::new("TOP")
                .with_size(18.0)
                .with_color(Color::from_srgb_u8(230, 48, 48))
                .with_background(Color::from_srgb_u8(230, 48, 48)),
            Transform::at(Vec3::ZERO),
        )
        .expect("near red label inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::new("L")
                .with_size(16.0)
                .with_color(Color::from_srgb_u8(32, 210, 96)),
            Transform::at(Vec3::new(-0.45, 0.32, 0.0)),
        )
        .expect("left green label inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::new("HI")
                .with_size(28.0)
                .with_color(Color::from_srgb_u8(240, 192, 32)),
            Transform::at(Vec3::new(0.45, -0.32, 0.0)),
        )
        .expect("right yellow label inserts");

    configure_headless_gpu_test_adapter();
    let mut renderer = Renderer::headless_gpu(160, 120).expect("HeadlessGpu renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("GPU label scene prepares");
    renderer
        .render(&scene, camera)
        .expect("GPU label scene renders");
    let frame = renderer.frame_rgba8().to_vec();

    let green = color_bounds(&frame, 160, 120, |pixel| {
        pixel[1] > 150 && pixel[0] < 90 && pixel[2] < 130
    })
    .expect("green label is visible");
    let yellow = color_bounds(&frame, 160, 120, |pixel| {
        pixel[0] > 180 && pixel[1] > 130 && pixel[2] < 100
    })
    .expect("yellow label is visible");
    let blue = color_bounds(&frame, 160, 120, |pixel| {
        pixel[2] > 150 && pixel[0] < 120 && pixel[1] < 130
    })
    .expect("far blue label ring is visible");
    let red = color_bounds(&frame, 160, 120, |pixel| {
        pixel[0] > 150 && pixel[1] < 110 && pixel[2] < 110
    })
    .expect("near red label is visible");

    assert!(
        green.center_x() < 70.0 && green.center_y() < 55.0,
        "green label should track its authored upper-left position: {green:?}"
    );
    assert!(
        yellow.center_x() > 90.0 && yellow.center_y() > 65.0,
        "yellow label should track its authored lower-right position: {yellow:?}"
    );
    assert!(
        yellow.width() > green.width() + 8 && yellow.height() >= green.height() + 8,
        "label size_px must produce distinct GPU glyph sizes: green={green:?} yellow={yellow:?}"
    );
    assert!(
        blue.width() > red.width() + 8 && blue.height() > red.height() + 8,
        "larger far blue label should leave a visible depth-test ring: blue={blue:?} red={red:?}"
    );
    let center = pixel_at(&frame, 160, 80, 60);
    assert!(
        center[0] > 150 && center[1] < 110 && center[2] < 110,
        "near red label must depth-test in front of far blue at the same screen position, center={center:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(&artifact_dir, "gpu-label-text", 160, 120, &frame);
}

#[test]
fn label_text_visual_proof_covers_multiple_sizes() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 4.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    for (index, (size, y)) in [(10.0, 0.45), (18.0, 0.0), (28.0, -0.45)]
        .into_iter()
        .enumerate()
    {
        scene
            .add_label(
                scene.root(),
                LabelDesc::new("TEXT")
                    .with_size(size)
                    .with_color(Color::from_srgb_u8(220, 240, 255))
                    .with_background(Color::from_srgb_u8(4, 8, 16)),
                Transform::at(Vec3::new(0.0, y, -(index as f32) * 0.4)),
            )
            .expect("label inserts");
    }

    let mut renderer = Renderer::headless(200, 140).expect("renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("multi-size labels prepare");
    renderer
        .render_active(&scene)
        .expect("multi-size labels render");
    let frame = renderer.frame_rgba8().to_vec();
    let rows = bright_row_counts(&frame, 200, 140);
    assert!(
        rows.iter().filter(|count| **count > 0).count() >= 24,
        "multi-size labels should cover enough scanlines to prove readable text, rows={rows:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "default-truetype-label-text-sizes",
        200,
        140,
        &frame,
    );
}

#[test]
fn label_text_long_billboards_keep_expected_width_and_sparse_glyphs() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 4.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let long_label = LabelDesc::new("BATTERY PACK TAIL CAP")
        .with_size(18.0)
        .with_color(Color::from_srgb_u8(245, 250, 255))
        .with_background(Color::from_srgb_u8(8, 14, 24));
    let expected_width = long_label.metrics().width_px;
    scene
        .add_label(scene.root(), long_label, Transform::at(Vec3::ZERO))
        .expect("long label inserts");

    let mut renderer = Renderer::headless(420, 140).expect("renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("long label scene prepares");
    renderer
        .render_active(&scene)
        .expect("long label scene renders");
    let frame = renderer.frame_rgba8().to_vec();

    let glyphs = color_bounds(&frame, 420, 140, |pixel| {
        pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 180
    })
    .expect("long label glyphs render");
    let background = color_bounds(&frame, 420, 140, |pixel| {
        pixel[0].saturating_add(pixel[1]).saturating_add(pixel[2]) > 2
            && pixel[0] < 60
            && pixel[1] < 80
            && pixel[2] < 80
    })
    .expect("long label background renders");

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "default-truetype-label-text-long",
        420,
        140,
        &frame,
    );

    assert!(
        (glyphs.width() as f32 - expected_width).abs() <= 10.0,
        "long label glyph bounds should track label metrics without overlap/doubling: expected_width={expected_width:.1} glyphs={glyphs:?}"
    );
    assert!(
        background.width() > glyphs.width() && background.height() > glyphs.height(),
        "background should frame, not overwrite, long label glyphs: background={background:?} glyphs={glyphs:?}"
    );
    assert!(
        glyphs.occupancy() < 0.70,
        "long label should remain glyph-shaped, not a smeared or doubled block: {glyphs:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn label_text_long_billboards_keep_expected_width_on_headless_gpu() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 4.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let long_label = LabelDesc::new("BATTERY PACK TAIL CAP")
        .with_size(18.0)
        .with_color(Color::from_srgb_u8(245, 250, 255))
        .with_background(Color::from_srgb_u8(8, 14, 24));
    let expected_width = long_label.metrics().width_px;
    scene
        .add_label(scene.root(), long_label, Transform::at(Vec3::ZERO))
        .expect("long label inserts");

    configure_headless_gpu_test_adapter();
    let mut renderer = Renderer::headless_gpu(420, 140).expect("HeadlessGpu renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("GPU long label scene prepares");
    renderer
        .render_active(&scene)
        .expect("GPU long label scene renders");
    let frame = renderer.frame_rgba8().to_vec();

    let glyphs = color_bounds(&frame, 420, 140, |pixel| {
        pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 180
    })
    .expect("GPU long label glyphs render");
    assert!(
        (glyphs.width() as f32 - expected_width).abs() <= 10.0,
        "GPU long label glyph bounds should track label metrics without overlap/doubling: expected_width={expected_width:.1} glyphs={glyphs:?}"
    );
    assert!(
        glyphs.occupancy() < 0.70,
        "GPU long label should remain glyph-shaped, not a smeared or doubled block: {glyphs:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "gpu-default-truetype-label-text-long",
        420,
        140,
        &frame,
    );
}

#[test]
fn label_text_remains_readable_across_camera_orbit() {
    for (name, camera_transform) in [
        ("front", Transform::at(Vec3::new(0.0, 0.0, 4.0))),
        (
            "side",
            Transform::at(Vec3::new(4.0, 0.0, 0.0)).looking_at(Vec3::ZERO, Vec3::Y),
        ),
    ] {
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::standard(),
                camera_transform,
            )
            .expect("camera inserts");
        scene.set_active_camera(camera).expect("camera activates");
        scene
            .add_label(
                scene.root(),
                LabelDesc::new("ORBIT")
                    .with_size(18.0)
                    .with_color(Color::from_srgb_u8(245, 250, 255))
                    .with_halo(Color::from_srgb_u8(40, 70, 110)),
                Transform::default(),
            )
            .expect("label inserts");

        let mut renderer = Renderer::headless(160, 120).expect("renderer builds");
        renderer.prepare(&mut scene).expect("orbit label prepares");
        renderer.render_active(&scene).expect("orbit label renders");
        let frame = renderer.frame_rgba8().to_vec();
        let bounds = bright_bounds(&frame, 160, 120).expect("orbit label visible");
        assert!(
            bounds.width() >= 24 && bounds.height() >= 10 && bounds.occupancy() < 0.9,
            "{name} orbit label should remain glyph-shaped and readable: {bounds:?}"
        );
    }
}

#[test]
fn label_text_many_label_prepare_benchmark_writes_artifact() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 5.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    for index in 0..128 {
        let col = index % 16;
        let row = index / 16;
        scene
            .add_label(
                scene.root(),
                LabelDesc::new(format!("L{index:03}"))
                    .with_size(12.0)
                    .with_color(Color::from_srgb_u8(220, 240, 255)),
                Transform::at(Vec3::new(
                    -1.2 + col as f32 * 0.16,
                    0.55 - row as f32 * 0.16,
                    0.0,
                )),
            )
            .expect("label inserts");
    }

    let mut renderer = Renderer::headless(240, 160).expect("renderer builds");
    let prepare_start = Instant::now();
    renderer.prepare(&mut scene).expect("many labels prepare");
    let prepare_ms = prepare_start.elapsed().as_secs_f64() * 1000.0;
    let render_start = Instant::now();
    renderer.render_active(&scene).expect("many labels render");
    let render_ms = render_start.elapsed().as_secs_f64() * 1000.0;
    let frame = renderer.frame_rgba8().to_vec();
    assert!(
        nonblack_pixel_count(&frame) > 1_000,
        "many-label scene should render visible glyph pixels"
    );
    assert!(
        prepare_ms < 5_000.0 && render_ms < 5_000.0,
        "many-label benchmark should remain bounded, prepare_ms={prepare_ms:.1} render_ms={render_ms:.1}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    fs::write(
        artifact_dir.join("default-truetype-label-benchmark.json"),
        format!(
            "{{\n  \"schema\": \"scena.label_text_benchmark.v1\",\n  \"labels\": 128,\n  \"prepare_ms\": {:.3},\n  \"render_ms\": {:.3},\n  \"nonblack_pixels\": {}\n}}\n",
            prepare_ms,
            render_ms,
            nonblack_pixel_count(&frame)
        ),
    )
    .expect("benchmark artifact writes");
}

#[derive(Debug, Clone, Copy)]
struct PixelBounds {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    pixels: u32,
}

impl PixelBounds {
    fn width(&self) -> u32 {
        self.max_x - self.min_x + 1
    }

    fn height(&self) -> u32 {
        self.max_y - self.min_y + 1
    }

    fn occupancy(&self) -> f32 {
        self.pixels as f32 / (self.width() * self.height()).max(1) as f32
    }

    #[cfg(feature = "scene-host")]
    fn center_x(&self) -> f32 {
        (self.min_x + self.max_x) as f32 * 0.5
    }

    #[cfg(feature = "scene-host")]
    fn center_y(&self) -> f32 {
        (self.min_y + self.max_y) as f32 * 0.5
    }
}

fn color_bounds(
    frame: &[u8],
    width: u32,
    height: u32,
    mut matches: impl FnMut(&[u8]) -> bool,
) -> Option<PixelBounds> {
    let mut bounds: Option<PixelBounds> = None;
    for y in 0..height {
        for x in 0..width {
            let offset = ((y * width + x) * 4) as usize;
            let pixel = frame.get(offset..offset + 4)?;
            if !matches(pixel) {
                continue;
            }
            match bounds.as_mut() {
                Some(bounds) => {
                    bounds.min_x = bounds.min_x.min(x);
                    bounds.min_y = bounds.min_y.min(y);
                    bounds.max_x = bounds.max_x.max(x);
                    bounds.max_y = bounds.max_y.max(y);
                    bounds.pixels += 1;
                }
                None => {
                    bounds = Some(PixelBounds {
                        min_x: x,
                        min_y: y,
                        max_x: x,
                        max_y: y,
                        pixels: 1,
                    });
                }
            }
        }
    }
    bounds
}

fn bright_row_counts(frame: &[u8], width: u32, height: u32) -> Vec<u32> {
    let mut rows = vec![0; height as usize];
    for y in 0..height {
        for x in 0..width {
            let offset = ((y * width + x) * 4) as usize;
            let Some(pixel) = frame.get(offset..offset + 4) else {
                continue;
            };
            if pixel[0] > 120 || pixel[1] > 120 || pixel[2] > 120 {
                rows[y as usize] += 1;
            }
        }
    }
    rows
}

fn bright_bounds(frame: &[u8], width: u32, height: u32) -> Option<PixelBounds> {
    color_bounds(frame, width, height, |pixel| {
        pixel[0] > 140 || pixel[1] > 140 || pixel[2] > 140
    })
}

fn nonblack_pixel_count(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0)
        .count()
}

fn render_truetype_aa_label(use_gpu: bool) -> (u32, u32, Vec<u8>) {
    let width = 220;
    let height = 120;
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 3.5)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_label(
            scene.root(),
            LabelDesc::truetype("BATTERY", font_bytes)
                .expect("TrueType label builds")
                .with_size(42.0)
                .with_color(Color::WHITE)
                .with_background(Color::DARK_GRAY),
            Transform::at(Vec3::ZERO),
        )
        .expect("TrueType AA label inserts");

    if use_gpu {
        #[cfg(feature = "scene-host")]
        {
            configure_headless_gpu_test_adapter();
            let mut renderer =
                Renderer::headless_gpu(width, height).expect("HeadlessGpu renderer builds");
            renderer
                .prepare(&mut scene)
                .expect("TrueType AA GPU scene prepares");
            renderer
                .render_active(&scene)
                .expect("TrueType AA GPU scene renders");
            return (width, height, renderer.frame_rgba8().to_vec());
        }
        #[cfg(not(feature = "scene-host"))]
        panic!("GPU label fixture requires the scene-host feature");
    }

    let mut renderer = Renderer::headless(width, height).expect("renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("TrueType AA CPU scene prepares");
    renderer
        .render_active(&scene)
        .expect("TrueType AA CPU scene renders");
    (width, height, renderer.frame_rgba8().to_vec())
}

fn intermediate_gray_pixel_count(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| {
            let [r, g, b, _a] = <[u8; 4]>::try_from(*pixel).expect("rgba chunk");
            let max_delta = r.abs_diff(g).max(r.abs_diff(b)).max(g.abs_diff(b));
            (16..=239).contains(&r)
                && (16..=239).contains(&g)
                && (16..=239).contains(&b)
                && max_delta <= 3
        })
        .count()
}

#[cfg(feature = "scene-host")]
#[derive(Debug)]
struct FrameDelta {
    max_channel_delta: u8,
    mean_channel_delta: f32,
}

#[cfg(feature = "scene-host")]
fn frame_delta_in_bounds(
    left: &[u8],
    right: &[u8],
    frame_width: u32,
    bounds: PixelBounds,
) -> FrameDelta {
    assert_eq!(left.len(), right.len());
    let mut max_channel_delta = 0_u8;
    let mut total = 0_u64;
    let mut count = 0_u64;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let offset = ((y * frame_width + x) * 4) as usize;
            for channel in 0..4 {
                let delta = left[offset + channel].abs_diff(right[offset + channel]);
                max_channel_delta = max_channel_delta.max(delta);
                total = total.saturating_add(u64::from(delta));
                count = count.saturating_add(1);
            }
        }
    }
    FrameDelta {
        max_channel_delta,
        mean_channel_delta: total as f32 / count.max(1) as f32,
    }
}

#[cfg(feature = "scene-host")]
fn mean_luminance_in_bounds(rgba: &[u8], frame_width: u32, bounds: PixelBounds) -> f32 {
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let pixel = pixel_at(rgba, frame_width, x, y);
            total += 0.2126 * f32::from(pixel[0])
                + 0.7152 * f32::from(pixel[1])
                + 0.0722 * f32::from(pixel[2]);
            count = count.saturating_add(1);
        }
    }
    total / count.max(1) as f32
}

#[cfg(feature = "scene-host")]
fn pixel_at(rgba: &[u8], width: u32, x: u32, y: u32) -> &[u8] {
    let start = ((y * width + x) * 4) as usize;
    &rgba[start..start + 4]
}

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact writes");
}

#[cfg(feature = "scene-host")]
fn write_ppm_crop_artifact(
    dir: &Path,
    name: &str,
    frame_width: u32,
    rgba: &[u8],
    crop: PixelBounds,
) {
    let width = crop.width();
    let height = crop.height();
    let mut ppm = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for y in crop.min_y..=crop.max_y {
        for x in crop.min_x..=crop.max_x {
            let offset = ((y * frame_width + x) * 4) as usize;
            ppm.extend_from_slice(&rgba[offset..offset + 3]);
        }
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM crop artifact writes");
}

fn system_test_font_path() -> PathBuf {
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Helvetica.ttf",
        "/Library/Fonts/Arial.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\calibri.ttf",
    ];
    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
        .expect("builder must provide a TrueType test font")
}
