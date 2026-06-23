use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use scena::{AntiAliasing, Assets, CameraKey, Renderer, Scene};

#[allow(dead_code)]
const LAVAPIPE_ICD: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRegion {
    pub const fn end_x(self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub const fn end_y(self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub fn shrink(self, inset: u32) -> Option<Self> {
        (self.width > inset.saturating_mul(2) && self.height > inset.saturating_mul(2)).then_some(
            Self {
                x: self.x.saturating_add(inset),
                y: self.y.saturating_add(inset),
                width: self.width.saturating_sub(inset.saturating_mul(2)),
                height: self.height.saturating_sub(inset.saturating_mul(2)),
            },
        )
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.end_x().min(other.end_x());
        let y1 = self.end_y().min(other.end_y());
        (x1 > x0 && y1 > y0).then_some(Self {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RenderBackend {
    Cpu,
    Gpu,
}

impl RenderBackend {
    #[allow(dead_code)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OwnedRgbaFrame {
    pub name: String,
    pub rgba8: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl OwnedRgbaFrame {
    #[allow(dead_code)]
    pub fn borrowed(&self) -> RgbaFrame<'_> {
        RgbaFrame::new(&self.name, &self.rgba8, self.width, self.height)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CpuGpuFramePair {
    pub cpu: OwnedRgbaFrame,
    pub gpu: OwnedRgbaFrame,
}

#[allow(dead_code)]
pub fn require_cpu_gpu_parity_adapter_or_skip(test_name: &str) -> bool {
    if std::env::var_os("SCENA_USE_GPU").is_none()
        && std::env::var_os("VK_ICD_FILENAMES").is_none()
        && !Path::new(LAVAPIPE_ICD).exists()
    {
        eprintln!(
            "skipping {test_name}; set SCENA_USE_GPU=1 or install lavapipe at {LAVAPIPE_ICD} to require CPU/GPU parity proof"
        );
        return false;
    }
    configure_lavapipe_adapter();
    true
}

#[allow(dead_code)]
pub fn configure_lavapipe_adapter() {
    if std::env::var_os("VK_ICD_FILENAMES").is_none() && Path::new(LAVAPIPE_ICD).exists() {
        // SAFETY: parity tests set the process adapter hint immediately before
        // constructing wgpu and do not read it concurrently themselves. This
        // turns installed lavapipe into an exercised GPU lane instead of a
        // silent skip.
        unsafe {
            std::env::set_var("VK_ICD_FILENAMES", LAVAPIPE_ICD);
        }
    }
}

#[allow(dead_code)]
pub fn renderer_for_backend(
    backend: RenderBackend,
    width: u32,
    height: u32,
    anti_aliasing: AntiAliasing,
) -> Renderer {
    let mut renderer = match backend {
        RenderBackend::Cpu => Renderer::headless(width, height).expect("CPU renderer builds"),
        RenderBackend::Gpu => Renderer::headless_gpu(width, height)
            .expect("HeadlessGpu renderer builds for required CPU/GPU parity proof"),
    };
    renderer.set_anti_aliasing(anti_aliasing);
    renderer
}

#[allow(dead_code)]
pub fn render_scene_frame(
    backend: RenderBackend,
    name: impl Into<String>,
    width: u32,
    height: u32,
    anti_aliasing: AntiAliasing,
    build: impl FnOnce(&mut Scene, &Assets) -> CameraKey,
) -> OwnedRgbaFrame {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = build(&mut scene, &assets);
    let mut renderer = renderer_for_backend(backend, width, height, anti_aliasing);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("scene render succeeds");
    OwnedRgbaFrame {
        name: name.into(),
        rgba8: renderer.frame_rgba8().to_vec(),
        width,
        height,
    }
}

#[allow(dead_code)]
pub fn render_scene_cpu_gpu_pair(
    name: &str,
    width: u32,
    height: u32,
    anti_aliasing: AntiAliasing,
    build: impl Fn(&mut Scene, &Assets) -> CameraKey + Copy,
) -> CpuGpuFramePair {
    CpuGpuFramePair {
        cpu: render_scene_frame(
            RenderBackend::Cpu,
            format!("{name}-cpu"),
            width,
            height,
            anti_aliasing,
            build,
        ),
        gpu: render_scene_frame(
            RenderBackend::Gpu,
            format!("{name}-gpu"),
            width,
            height,
            anti_aliasing,
            build,
        ),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RgbaFrame<'a> {
    pub name: &'a str,
    pub rgba8: &'a [u8],
    pub width: u32,
    pub height: u32,
}

impl<'a> RgbaFrame<'a> {
    pub fn new(name: &'a str, rgba8: &'a [u8], width: u32, height: u32) -> Self {
        assert_eq!(
            rgba8.len(),
            (width as usize)
                .saturating_mul(height as usize)
                .saturating_mul(4),
            "{name} frame byte length must match width/height"
        );
        Self {
            name,
            rgba8,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelDelta {
    pub max_channel_delta: u8,
    pub mean_channel_delta: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct RegionStructure {
    pub sobel_luminance_energy: f32,
    pub luminance_range: f32,
    pub unique_luma_levels: usize,
    pub foreground_fraction: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ParityComparison {
    pub rmse: f32,
    pub channel_delta: ChannelDelta,
    pub left_structure: RegionStructure,
    pub right_structure: RegionStructure,
}

#[derive(Debug, Clone)]
pub struct ParitySweepRecord {
    pub name: String,
    pub reference: String,
    pub candidate: String,
    pub region: PixelRegion,
    pub comparison: ParityComparison,
}

#[derive(Debug, Clone)]
pub struct ParitySweep {
    schema: String,
    records: Vec<ParitySweepRecord>,
}

impl ParitySweep {
    pub fn new(schema: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            records: Vec::new(),
        }
    }

    pub fn compare_region(
        &mut self,
        name: impl Into<String>,
        reference: RgbaFrame<'_>,
        candidate: RgbaFrame<'_>,
        region: PixelRegion,
    ) -> ParityComparison {
        let comparison = compare_frames_in_region(reference, candidate, region);
        self.records.push(ParitySweepRecord {
            name: name.into(),
            reference: reference.name.to_owned(),
            candidate: candidate.name.to_owned(),
            region,
            comparison,
        });
        comparison
    }

    pub fn records(&self) -> &[ParitySweepRecord] {
        &self.records
    }

    pub fn write_json(&self, path: &Path, extra_fields: &[(&str, String)]) {
        let mut json = String::new();
        writeln!(&mut json, "{{").expect("write to string");
        writeln!(&mut json, "  \"schema\": \"{}\",", self.schema).expect("write to string");
        for (key, value) in extra_fields {
            writeln!(&mut json, "  \"{key}\": {value},").expect("write to string");
        }
        writeln!(&mut json, "  \"records\": [").expect("write to string");
        for (index, record) in self.records.iter().enumerate() {
            let comma = if index + 1 == self.records.len() {
                ""
            } else {
                ","
            };
            writeln!(
                &mut json,
                "    {{ \"name\": \"{}\", \"reference\": \"{}\", \"candidate\": \"{}\", \"rmse\": {:.5}, \"max_channel_delta\": {}, \"mean_channel_delta\": {:.5}, \"reference_sobel_energy\": {:.5}, \"candidate_sobel_energy\": {:.5}, \"reference_luminance_range\": {:.5}, \"candidate_luminance_range\": {:.5}, \"reference_unique_luma_levels\": {}, \"candidate_unique_luma_levels\": {}, \"reference_foreground_fraction\": {:.5}, \"candidate_foreground_fraction\": {:.5}, \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }} }}{}",
                record.name,
                record.reference,
                record.candidate,
                record.comparison.rmse,
                record.comparison.channel_delta.max_channel_delta,
                record.comparison.channel_delta.mean_channel_delta,
                record.comparison.left_structure.sobel_luminance_energy,
                record.comparison.right_structure.sobel_luminance_energy,
                record.comparison.left_structure.luminance_range,
                record.comparison.right_structure.luminance_range,
                record.comparison.left_structure.unique_luma_levels,
                record.comparison.right_structure.unique_luma_levels,
                record.comparison.left_structure.foreground_fraction,
                record.comparison.right_structure.foreground_fraction,
                record.region.x,
                record.region.y,
                record.region.width,
                record.region.height,
                comma
            )
            .expect("write to string");
        }
        writeln!(&mut json, "  ]").expect("write to string");
        writeln!(&mut json, "}}").expect("write to string");
        fs::write(path, json).expect("parity sweep artifact writes");
    }
}

pub fn compare_frames_in_region(
    left: RgbaFrame<'_>,
    right: RgbaFrame<'_>,
    region: PixelRegion,
) -> ParityComparison {
    assert_eq!(
        (left.width, left.height),
        (right.width, right.height),
        "CPU/GPU parity frames must have identical dimensions"
    );
    assert_eq!(left.rgba8.len(), right.rgba8.len());
    ParityComparison {
        rmse: frame_rmse_in_region(left.rgba8, right.rgba8, left.width, region),
        channel_delta: frame_delta_in_region(left.rgba8, right.rgba8, left.width, region),
        left_structure: structure_metrics_in_region(left, region),
        right_structure: structure_metrics_in_region(right, region),
    }
}

pub fn frame_delta_in_region(
    left: &[u8],
    right: &[u8],
    frame_width: u32,
    region: PixelRegion,
) -> ChannelDelta {
    assert_eq!(left.len(), right.len());
    let mut max_channel_delta = 0_u8;
    let mut total = 0_u64;
    let mut count = 0_u64;
    for y in region.y..region.end_y() {
        for x in region.x..region.end_x() {
            let offset = ((y * frame_width + x) * 4) as usize;
            for channel in 0..3 {
                let delta = left[offset + channel].abs_diff(right[offset + channel]);
                max_channel_delta = max_channel_delta.max(delta);
                total = total.saturating_add(u64::from(delta));
                count = count.saturating_add(1);
            }
        }
    }
    ChannelDelta {
        max_channel_delta,
        mean_channel_delta: total as f32 / count.max(1) as f32,
    }
}

pub fn frame_rmse_in_region(
    left: &[u8],
    right: &[u8],
    frame_width: u32,
    region: PixelRegion,
) -> f32 {
    assert_eq!(left.len(), right.len());
    let mut sum_squares = 0.0_f64;
    let mut count = 0_u64;
    for y in region.y..region.end_y() {
        for x in region.x..region.end_x() {
            let offset = ((y * frame_width + x) * 4) as usize;
            for channel in 0..3 {
                let delta = (f64::from(left[offset + channel])
                    - f64::from(right[offset + channel]))
                    / 255.0;
                sum_squares += delta * delta;
                count = count.saturating_add(1);
            }
        }
    }
    (sum_squares / count.max(1) as f64).sqrt() as f32
}

pub fn structure_metrics_in_region(frame: RgbaFrame<'_>, region: PixelRegion) -> RegionStructure {
    let mut foreground_pixels = 0usize;
    let mut min_luma = f32::INFINITY;
    let mut max_luma = f32::NEG_INFINITY;
    let mut unique_luma = BTreeSet::new();
    for y in region.y..region.end_y().min(frame.height) {
        for x in region.x..region.end_x().min(frame.width) {
            let offset = ((y as usize) * (frame.width as usize) + x as usize) * 4;
            let Some(pixel) = frame.rgba8.get(offset..offset + 4) else {
                continue;
            };
            if pixel[3] == 0 || pixel[..3].iter().all(|channel| *channel >= 248) {
                continue;
            }
            foreground_pixels += 1;
            let luma = srgb_luminance_u8(pixel);
            min_luma = min_luma.min(luma);
            max_luma = max_luma.max(luma);
            unique_luma.insert((luma * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    let region_pixels = (region.width as usize).saturating_mul(region.height as usize);
    RegionStructure {
        sobel_luminance_energy: sobel_luminance_energy_in_region(
            frame.rgba8,
            frame.width,
            frame.height,
            region,
        ),
        luminance_range: if min_luma.is_finite() && max_luma.is_finite() {
            max_luma - min_luma
        } else {
            0.0
        },
        unique_luma_levels: unique_luma.len(),
        foreground_fraction: foreground_pixels as f32 / region_pixels.max(1) as f32,
    }
}

pub fn sobel_luminance_energy_in_region(
    rgba: &[u8],
    frame_width: u32,
    frame_height: u32,
    region: PixelRegion,
) -> f32 {
    let min_x = region.x.max(1);
    let min_y = region.y.max(1);
    let max_x = region.end_x().min(frame_width.saturating_sub(1));
    let max_y = region.end_y().min(frame_height.saturating_sub(1));
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in min_y..max_y {
        for x in min_x..max_x {
            let l = |ox: i32, oy: i32| {
                let sx = (x as i32 + ox).clamp(0, frame_width.saturating_sub(1) as i32) as u32;
                let sy = (y as i32 + oy).clamp(0, frame_height.saturating_sub(1) as i32) as u32;
                linear_luminance_at(rgba, frame_width, sx, sy)
            };
            let gx = -l(-1, -1) + l(1, -1) - 2.0 * l(-1, 0) + 2.0 * l(1, 0) - l(-1, 1) + l(1, 1);
            let gy = -l(-1, -1) - 2.0 * l(0, -1) - l(1, -1) + l(-1, 1) + 2.0 * l(0, 1) + l(1, 1);
            total += (gx * gx + gy * gy).sqrt();
            count = count.saturating_add(1);
        }
    }
    total / count.max(1) as f32
}

fn linear_luminance_at(rgba: &[u8], frame_width: u32, x: u32, y: u32) -> f32 {
    let offset = ((y * frame_width + x) * 4) as usize;
    if offset + 2 >= rgba.len() {
        return 0.0;
    }
    let r = srgb_to_linear(rgba[offset]);
    let g = srgb_to_linear(rgba[offset + 1]);
    let b = srgb_to_linear(rgba[offset + 2]);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn srgb_luminance_u8(pixel: &[u8]) -> f32 {
    (0.2126 * f32::from(pixel[0]) + 0.7152 * f32::from(pixel[1]) + 0.0722 * f32::from(pixel[2]))
        / 255.0
}

fn srgb_to_linear(value: u8) -> f32 {
    let c = f32::from(value) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
