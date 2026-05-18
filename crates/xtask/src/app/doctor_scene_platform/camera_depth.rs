use crate::app::prelude::*;

pub(crate) fn check_reversed_z_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum CapabilityStatus",
            "Supported",
            "FeatureDisabled",
            "pub reversed_z_depth: CapabilityStatus",
            "const fn reversed_z_depth_status",
            "Backend::WebGl2",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/lib.rs",
        &["CapabilityStatus"],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/render/prepare/stats.rs",
        &[
            "reversed_z: bool",
            "capabilities.reversed_z_depth == CapabilityStatus::Supported",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/render/gpu/depth.rs",
        &[
            "reversed_z: bool",
            "wgpu::CompareFunction::GreaterEqual",
            "clear_depth: if reversed_z { 0.0 } else { 1.0 }",
            "wgpu::LoadOp::Clear(resources.clear_depth)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "capability_matrix_reports_reversed_z_depth_support_and_webgl2_fallback",
            "CapabilityStatus::Supported",
            "CapabilityStatus::FeatureDisabled",
            "Backend::WebGl2",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "docs/specs/public-api.md",
        &[
            "pub reversed_z_depth: CapabilityStatus",
            "Capabilities::reversed_z_depth",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Reversed-Z support", "ARCH-REVERSED-Z"],
    );
}

pub(crate) fn check_webgl2_depth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "src/diagnostics/capabilities.rs",
        &[
            "pub fn diagnostics(self) -> Vec<Diagnostic>",
            "self.backend == Backend::WebGl2",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "near/far ranges",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "src/diagnostics/diagnostic.rs",
        &["WebGl2DepthCompatibility"],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "webgl2_depth_capability_reports_structured_compatibility_warning",
            "Capabilities::for_attached_gpu_backend(Backend::WebGl2).diagnostics()",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "near/far",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "docs/specs/public-api.md",
        &[
            "Capabilities::diagnostics()",
            "DiagnosticCode::WebGl2DepthCompatibility",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["WebGL2 depth compatibility warnings", "ARCH-WEBGL2-DEPTH"],
    );
}

pub(crate) fn check_m2_leak_stats_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M2-LEAK-STATS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "m2_resource_counters_return_to_baseline_after_empty_prepare",
            "environment_cubemaps",
            "environment_prefilter_passes",
            "environment_brdf_luts",
            "shadow_maps",
            "depth_prepass_passes",
            "depth_prepass_draws",
            "released.textures, baseline.textures",
            "released.pending_destructions, baseline.pending_destructions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M2-LEAK-STATS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "m2_resource_counters_return_to_baseline_after_empty_prepare",
            "ARCH-M2-LEAK-STATS",
        ],
    );
}

pub(crate) fn check_camera_depth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene.rs",
        &[
            "mod camera;",
            "pub use camera::{Camera, DepthRange, OrthographicCamera, PerspectiveCamera}",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene/camera.rs",
        &[
            "pub enum Camera",
            "pub struct PerspectiveCamera",
            "pub struct OrthographicCamera",
            "pub struct DepthRange",
            "aspect: 0.0",
            "pub const fn with_aspect(mut self, aspect: f32) -> Self",
            "pub const fn new(near: f32, far: f32) -> Self",
            "pub const fn fit_sphere(center_distance: f32, radius: f32) -> Self",
            "pub const fn contains_interval(self, near: f32, far: f32) -> bool",
            "pub const fn with_depth_range(mut self, range: DepthRange) -> Self",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/lib.rs",
        &["DepthRange", "PerspectiveCamera", "OrthographicCamera"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "camera_depth_fit_helpers_cover_unit_cube_reference_distances",
            "DepthRange::fit_sphere",
            "with_depth_range",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Camera depth-range and depth-fit helpers",
            "DepthRange::fit_sphere",
        ],
    );
}

pub(crate) fn check_clipping_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/scene.rs",
        &["pub struct ClippingPlaneKey"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/scene/clipping.rs",
        &[
            "pub struct ClippingPlane",
            "pub struct ClippingPlaneSet",
            "pub fn add_clipping_plane",
            "pub fn set_clipping_planes",
            "pub(crate) fn active_clipping_plane_values",
            "pub fn contains(self, point: Vec3) -> bool",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/diagnostics.rs",
        &["ClippingPlaneNotFound"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/lib.rs",
        &["ClippingPlane", "ClippingPlaneKey", "ClippingPlaneSet"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/render.rs",
        &[
            "clipping_planes: Vec<ClippingPlane>",
            "prepared.clipping_planes.clone()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/render/prepare_lifecycle.rs",
        &["scene.active_clipping_plane_values().collect()"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/render/cpu.rs",
        &[
            "clipping_planes: &[ClippingPlane]",
            "mix_position",
            "is_clipped",
            "plane.contains(position)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "clipping_plane_set_clips_rendered_output_half_space",
            "ClippingPlane::new",
            "ClippingPlaneSet::new().with_plane",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "docs/specs/public-api.md",
        &[
            "pub struct ClippingPlaneKey",
            "dot(normal, position) + distance >= 0",
            "ClippingPlaneNotFound",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["ClippingPlane", "ARCH-CLIPPING"],
    );
}

pub(crate) fn check_origin_shift_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/scene.rs",
        &["origin_shift: Vec3"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/scene/origin.rs",
        &[
            "pub fn set_origin_shift",
            "pub fn origin_shift(&self) -> Vec3",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/render/prepare.rs",
        &[
            "let origin_shift = scene.origin_shift()",
            "prepared_primitive",
            "transform_position",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/render/prepare/diagnostics.rs",
        &["subtract_vec3", "relative_translation"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "origin_shift_keeps_large_offset_renderable_visible_without_precision_warning",
            "scene.set_origin_shift",
            "DiagnosticCode::LargeScenePrecisionRisk",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "docs/specs/public-api.md",
        &["pub fn set_origin_shift", "large-world"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Camera-relative rendering or origin-shift support",
            "ARCH-ORIGIN-SHIFT",
        ],
    );
}
