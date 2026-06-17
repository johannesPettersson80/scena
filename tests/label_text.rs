#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use scena::{Color, LabelDesc, PerspectiveCamera, Renderer, Scene, Transform, Vec3};

#[test]
fn label_desc_reports_stable_font_metrics_and_style_options() {
    let label = LabelDesc::bitmap("Hi")
        .with_size(14.0)
        .with_background(Color::from_srgb_u8(8, 12, 20))
        .with_halo(Color::from_srgb_u8(220, 240, 255));

    let metrics = label.metrics();
    assert_eq!(metrics.glyph_count, 2);
    assert_eq!(metrics.width_px, 22.0);
    assert_eq!(metrics.height_px, 14.0);
    assert_eq!(metrics.baseline_px, 12.0);
    assert_eq!(label.background(), Some(Color::from_srgb_u8(8, 12, 20)));
    assert_eq!(label.halo(), Some(Color::from_srgb_u8(220, 240, 255)));
}

#[test]
fn label_desc_translucent_explicit_colors_fail_closed_until_transparent_path_exists() {
    for (name, label) in [
        (
            "text color",
            LabelDesc::bitmap("A").with_color(Color::from_linear_rgba(1.0, 1.0, 1.0, 0.5)),
        ),
        (
            "background",
            LabelDesc::bitmap("A").with_background(Color::from_linear_rgba(0.0, 0.0, 0.0, 0.5)),
        ),
        (
            "halo",
            LabelDesc::bitmap("A").with_halo(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)),
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
fn label_desc_truetype_font_changes_metrics_and_rendered_coverage() {
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let bitmap = LabelDesc::bitmap("AVAV")
        .with_size(28.0)
        .with_color(Color::RED);
    let truetype = LabelDesc::truetype("AVAV", font_bytes)
        .expect("TrueType font loads")
        .with_size(28.0)
        .with_color(Color::GREEN);

    let bitmap_metrics = bitmap.metrics();
    let truetype_metrics = truetype.metrics();
    assert_eq!(bitmap_metrics.glyph_count, 4);
    assert_eq!(truetype_metrics.glyph_count, 4);
    assert!(
        (bitmap_metrics.width_px - truetype_metrics.width_px).abs() >= 4.0,
        "loaded TrueType metrics must not silently use bitmap metrics: bitmap={bitmap_metrics:?} truetype={truetype_metrics:?}"
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
            bitmap,
            Transform::at(Vec3::new(-0.45, 0.0, 0.0)),
        )
        .expect("bitmap label inserts");
    scene
        .add_label(
            scene.root(),
            truetype,
            Transform::at(Vec3::new(0.45, 0.0, 0.0)),
        )
        .expect("truetype label inserts");

    let mut renderer = Renderer::headless(220, 120).expect("renderer builds");
    renderer.prepare(&mut scene).expect("font scene prepares");
    renderer.render_active(&scene).expect("font scene renders");
    let frame = renderer.frame_rgba8().to_vec();

    let bitmap_bounds = color_bounds(&frame, 220, 120, |pixel| {
        pixel[0] > 90
            && pixel[0] > pixel[1].saturating_mul(2)
            && pixel[0] > pixel[2].saturating_mul(2)
    })
    .expect("bitmap label renders");
    let truetype_bounds = color_bounds(&frame, 220, 120, |pixel| {
        pixel[1] > 80
            && pixel[1] > pixel[0].saturating_mul(2)
            && pixel[1] > pixel[2].saturating_mul(2)
    })
    .expect("truetype label renders");
    assert!(
        (bitmap_bounds.width() as i32 - truetype_bounds.width() as i32).abs() >= 4
            || (bitmap_bounds.height() as i32 - truetype_bounds.height() as i32).abs() >= 2
            || (bitmap_bounds.occupancy() - truetype_bounds.occupancy()).abs() >= 0.05,
        "loaded TrueType coverage must be distinguishable from embedded bitmap coverage: bitmap={bitmap_bounds:?} truetype={truetype_bounds:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "truetype-vs-bitmap-label-text",
        220,
        120,
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
fn label_text_renders_glyph_cells_with_pixel_stable_billboards() {
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
            LabelDesc::bitmap("HI")
                .with_size(14.0)
                .with_color(Color::RED)
                .with_halo(Color::from_srgb_u8(255, 220, 220)),
            Transform::at(Vec3::new(-0.45, 0.2, 0.0)),
        )
        .expect("near label inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::bitmap("HI")
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
    write_ppm_artifact(&artifact_dir, "bitmap-label-text", 160, 120, &frame);
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
                LabelDesc::bitmap("TEXT")
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
        rows.iter().filter(|count| **count > 0).count() >= 42,
        "multi-size labels should cover enough scanlines to prove readable text, rows={rows:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/labels");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(&artifact_dir, "bitmap-label-text-sizes", 200, 140, &frame);
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
                LabelDesc::bitmap("ORBIT")
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
                LabelDesc::bitmap(format!("L{index:03}"))
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
        artifact_dir.join("bitmap-label-benchmark.json"),
        format!(
            "{{\n  \"schema\": \"scena.label_text_benchmark.v1\",\n  \"labels\": 128,\n  \"prepare_ms\": {:.3},\n  \"render_ms\": {:.3},\n  \"nonblack_pixels\": {}\n}}\n",
            prepare_ms,
            render_ms,
            nonblack_pixel_count(&frame)
        ),
    )
    .expect("benchmark artifact writes");
}

#[derive(Debug)]
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

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact writes");
}

fn system_test_font_path() -> PathBuf {
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
        .expect("builder must provide a TrueType test font")
}
