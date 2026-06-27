use crate::app::prelude::*;

pub(super) fn check_browser_material_preset_proof(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/browser_probe/workflows/pbr/material_presets.rs",
        &[
            "material_presets_scene",
            "material_preset_showcase",
            "browser-pbr-material-preset-expanded-set",
            "webgl2_smooth_metal_sample_floor",
            "scene-color-ior-thickness-rough-blur-sorted-transparency",
            "glass_pixel_probes",
            "glass_pixel_probe_viewport",
            "/demo/samples/environment/white_studio_03_1k.hdr",
            "showcase_geometry",
            "source_surfaces",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "assertMaterialPresetProof",
            "pbr-material-presets",
            "material_preset_glass_pixels",
            "browser-glass-pixel-probes",
            "structured glass pixels behind clear/frosted glass",
            "webgl2_smooth_metal_sample_floor < 96",
            "/demo/samples/environment/white_studio_03_1k.hdr",
            "single-shape grid",
            "Assets::material_presets()",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/browser/m6_rust_wasm_renderer_probe_page.js",
        &[
            "materialPresetGlassPixelProof",
            "glass_pixel_probes",
            "samplePixelBuffer",
            "browser-glass-pixel-probes",
            "readRenderedPixelBuffer",
        ],
    );
}
