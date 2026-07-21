use crate::app::prelude::*;

const CPU_RENDER_COMMAND: &str = "bash scripts/release_lane_command.sh headless-cpu cargo test \
    --test examples_visual_proof \
    q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame -- --exact";
const CPU_EVALUATOR_COMMAND: &str = "bash scripts/release_lane_command.sh headless-cpu node \
    scripts/evaluate_round_e_cpu_materials.cjs";
const WEBGL_EVALUATOR_COMMAND: &str = "bash scripts/release_lane_command.sh \
    linux-webgl2-chromium npm run cloudflare:materials -- \
    http://127.0.0.1:18104/proof/?sample=material-presets";

pub(crate) fn check_q02_round_e_material_proof(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "tests/m8_visual_proof.rs",
        &[
            "m8_khr_material_visual_oracle_rejects_disabled_and_wrong_direction_mutations",
            "KHR_VISIBLE_CHANNEL_DELTA: u8 = 4",
            "KHR_NUMERICAL_RMSE_MAX: f32 = 1.1",
            "KHR_EFFECT_ALIGNMENT_MIN: f32 = 0.9",
            "two_lsb_effect_nudge_rejected",
            "wrong_direction_rejected",
            "one_lsb_noise_passed",
            "anisotropy-light-left",
            "anisotropy-light-right",
            "khr-material-feature-proof.json",
            "q02-khr-material-feature-mutation-proof",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "scripts/round_e_material_evaluator.cjs",
        &[
            "round-e-shared-material-threshold-evaluator",
            "cropRoundEMaterialTiles",
            "evaluateRoundEMaterialTiles",
            "thresholdsForSurface",
            "localTextureVariance",
            "highlightAspectRatio",
            "darkTargetOffset",
            "sobelEdgeEnergy",
            "meanDeltaE2000",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "scripts/tests/round_e_material_evaluator_test.cjs",
        &[
            "\"synthetic-contract\"",
            "\"live-cpu-headless\"",
            "\"live-webgpu-chromium\"",
            "\"flat chrome\"",
            "\"isotropic brushed metal\"",
            "\"identical neighbors\"",
            "\"lost clearcoat\"",
            "\"missing transmission/refraction\"",
            "\"removed texture variance\"",
            "chrome_specular_dynamic_range",
            "brushed_steel_anisotropy",
            "neighbor_delta",
            "clearcoat_lobe",
            "clear_glass_refraction",
            "leather_texture_variance",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "scripts/evaluate_round_e_cpu_materials.cjs",
        &[
            "./round_e_material_evaluator.cjs",
            "scena.q02.round_e_cpu_material_proof.v1",
            "live-cpu-round-e-shared-threshold-evaluation",
            "surface: \"live-cpu-headless\"",
            "requireReferenceDelta: false",
            "attachReleaseArtifactProvenance",
            "status: evaluation.status === \"pass\" ? \"passed\" : \"failed\"",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "tests/examples_visual_proof.rs",
        &[
            "q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame",
            "scena.q02.live_cpu_material_frame.v1",
            "live-cpu-rendered-round-e-material-showcase",
            "Renderer::headless",
            "material_preset_showcase()",
            "round-e-cpu-material-proof/live-frame.png",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "scripts/probe_cloudflare_material_presets.mjs",
        &[
            "./round_e_material_evaluator.cjs",
            "scena.q02.round_e_webgl2_material_proof.v1",
            "round-e-cloudflare-material-proof",
            "surface: \"live-webgl2-chromium\"",
            "requireReferenceDelta: true",
            "reference_delta_gate: \"hard\"",
            "attachReleaseArtifactProvenance",
            "status: errors.length === 0 ? \"passed\" : \"failed\"",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "evaluateRequiredWebgpuMaterialProof(materialPresetResult)",
            "writeRequiredWebgpuMaterialArtifact",
            "renderer-owned RGBA8 readback",
            "surface: \"live-webgpu-chromium\"",
            "requireReferenceDelta: false",
            "scena.q02.round_e_webgpu_material_proof.v1",
            "required-live-webgpu-round-e-shared-threshold-evaluation",
            "attachReleaseArtifactProvenance",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "tests/visual/references/round_e_material_thresholds.toml",
        &[
            "[live_cpu_headless.brushed_steel]",
            "[live_cpu_headless.clearcoat_plastic]",
            "[live_cpu_headless.leather]",
            "[live_cpu_headless.rubber]",
            "[live_webgl2_chromium.leather]",
            "[live_webgpu_chromium.brushed_steel]",
            "[live_webgpu_chromium.leather]",
            "[live_webgpu_chromium.rubber]",
        ],
    );
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        require_contains(
            root,
            findings,
            "Q02-ROUND-E-MATERIALS",
            workflow,
            &[
                CPU_RENDER_COMMAND,
                CPU_EVALUATOR_COMMAND,
                "SCENA_BROWSER_BACKENDS: webgpu",
                "bash scripts/release_lane_command.sh linux-webgpu-chromium npm run browser:q02-materials",
                "bash scripts/release_lane_command.sh linux-webgpu-chromium npm run browser:m6",
                WEBGL_EVALUATOR_COMMAND,
            ],
        );
    }
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "crates/xtask/src/app/release/lane_artifacts.rs",
        &[
            "round-e-cpu-material-proof/live-frame.png",
            "round-e-cpu-material-proof/live-cpu-frame.json",
            "round-e-cpu-material-proof.json",
            "round-e-cloudflare-material-proof.json",
            "round-e-cloudflare-material-proof/canvas.png",
            "round-e-webgpu-material-proof/live-frame.png",
            "round-e-webgpu-material-proof/result.json",
            "npm run browser:q02-materials",
            "q02_material_result_passes",
            "node scripts/evaluate_round_e_cpu_materials.cjs",
            "npm run cloudflare:materials -- http://127.0.0.1:18104/proof/?sample=material-presets",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "crates/xtask/src/app/release/round_e_material_results.rs",
        &[
            "q02_material_result_passes",
            "round-e-shared-material-threshold-evaluator",
            "live-cpu-headless",
            "live-webgl2-chromium",
            "live-webgpu-chromium",
            "passed_reference_delta",
            "renderer-owned-gpu-copy",
            "source_checksums_valid",
            "live_frame_valid",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "crates/xtask/src/app/release/review_artifacts.rs",
        &[
            "round-e-cpu-material-proof/live-frame.png",
            "round-e-cpu-material-proof.json",
            "round-e-cloudflare-material-proof.json",
            "round-e-webgpu-material-proof/result.json",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "docs/rendering.md",
        &[
            "KHR material visual-proof contract",
            "two-LSB effect nudge",
            "Directional anisotropy",
            "under two light directions",
            "khr-material-feature-proof.json",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "README.md",
        &[
            "KHR material feature proofs",
            "disabled and inverted-effect mutations",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "CHANGELOG.md",
        &[
            "Strengthen the KHR material visual proofs",
            "two anisotropy light directions",
        ],
    );
    require_contains(
        root,
        findings,
        "Q02-ROUND-E-MATERIALS",
        "docs/release-notes/v1.8.0.md",
        &[
            "one- or two-LSB change",
            "disabled and inverted-effect mutations",
        ],
    );
}
