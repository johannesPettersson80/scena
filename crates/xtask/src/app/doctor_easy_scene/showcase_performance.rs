use crate::app::prelude::*;

const DEMO_HDR_PATH: &str = "demo/samples/environment/white_studio_03_1k.hdr";
const DEMO_HDR_SIDECAR_PATH: &str = "demo/samples/environment/white_studio_03_1k.hdr.prefilter.bin";
const PUBLIC_SHOWCASE_WASM_PATH: &str = "demo/pkg/scena_bg.wasm";
const PROOF_HARNESS_WASM_PATH: &str = "demo/proof/pkg/scena_bg.wasm";
const PUBLIC_SHOWCASE_WASM_BASELINE_RAW_BYTES: u64 = 4_014_796;
const PUBLIC_SHOWCASE_WASM_BASELINE_BROTLI_BYTES: u64 = 1_080_427;
const PROOF_HARNESS_WASM_BASELINE_RAW_BYTES: u64 = 4_564_702;
const PROOF_HARNESS_WASM_BASELINE_BROTLI_BYTES: u64 = 1_221_378;
const PUBLIC_SHOWCASE_WASM_RAW_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PUBLIC_SHOWCASE_WASM_BASELINE_RAW_BYTES);
const PUBLIC_SHOWCASE_WASM_BROTLI_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PUBLIC_SHOWCASE_WASM_BASELINE_BROTLI_BYTES);
const PROOF_HARNESS_WASM_RAW_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PROOF_HARNESS_WASM_BASELINE_RAW_BYTES);
const PROOF_HARNESS_WASM_BROTLI_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PROOF_HARNESS_WASM_BASELINE_BROTLI_BYTES);

const fn ten_percent_growth_budget(baseline: u64) -> u64 {
    baseline + baseline / 10
}

pub(super) fn check_showcase_performance_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_demo_hdr_sidecar_current(root, findings);
    check_wasm_size_budget(root, findings);
    require_contains(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        "crates/xtask/src/app/prerender_environment.rs",
        &[
            "run_prerender_environment",
            "precompute_environment_sidecar",
            "EnvironmentSidecarProfile::InteractiveWebGl2",
            ".prefilter.bin",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        "src/assets/environment_loading.rs",
        &[
            "try_load_environment_sidecar",
            "sidecar_path_for_environment",
            "EnvironmentPrefilterSidecar::parse",
            "from_equirectangular_hdr_sidecar_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        "src/render/prepare/environment.rs",
        &[
            "prefilter_sidecar",
            "load_prefilter_sidecar",
            "prefilter_specular_cubemap_mips_with_quality",
            "precompute_environment_sidecar",
        ],
    );
    require_contains(
        root,
        findings,
        "PUBLIC-SHOWCASE-WASM-SIZE",
        "scripts/build_demo_wasm.js",
        &[
            "demo/proof/pkg",
            "proof-harness,browser-probe",
            "--strip-debug",
            "--strip-dwarf",
            "--strip-producers",
        ],
    );
}

fn check_demo_hdr_sidecar_current(root: &Path, findings: &mut Vec<Finding>) {
    let hdr_path = root.join(DEMO_HDR_PATH);
    let sidecar_path = root.join(DEMO_HDR_SIDECAR_PATH);
    if !sidecar_path.exists() {
        findings.push(Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!("{DEMO_HDR_SIDECAR_PATH} must exist and be generated from {DEMO_HDR_PATH}"),
        ));
        return;
    }
    let hdr_sha = match sha256_hex(&hdr_path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "DEMO-HDR-SIDECAR-CURRENT",
                format!("could not hash {DEMO_HDR_PATH}: {error}"),
            ));
            return;
        }
    };
    let sidecar_bytes = match fs::read(&sidecar_path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "DEMO-HDR-SIDECAR-CURRENT",
                format!("could not read {DEMO_HDR_SIDECAR_PATH}: {error}"),
            ));
            return;
        }
    };
    let header = match scena::parse_sidecar_header(DEMO_HDR_SIDECAR_PATH, &sidecar_bytes) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "DEMO-HDR-SIDECAR-CURRENT",
                format!("could not parse {DEMO_HDR_SIDECAR_PATH} header: {error:?}"),
            ));
            return;
        }
    };
    if header.source_sha256_hex() != hdr_sha {
        findings.push(Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!(
                "{DEMO_HDR_SIDECAR_PATH} source HDR SHA mismatch: header {}, actual {hdr_sha}",
                header.source_sha256_hex()
            ),
        ));
    }
    if header.profile_name() != "InteractiveWebGl2" {
        findings.push(Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!(
                "{DEMO_HDR_SIDECAR_PATH} must use InteractiveWebGl2, got {}",
                header.profile_name()
            ),
        ));
    }
}

fn check_wasm_size_budget(root: &Path, findings: &mut Vec<Finding>) {
    let public_path = root.join(PUBLIC_SHOWCASE_WASM_PATH);
    let proof_path = root.join(PROOF_HARNESS_WASM_PATH);
    let public_exists = public_path.exists();
    let proof_exists = proof_path.exists();

    if !public_exists && !proof_exists {
        return;
    }

    if public_exists != proof_exists {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "showcase WASM size budgets require both generated bundles to exist: \
                 {PUBLIC_SHOWCASE_WASM_PATH} exists={public_exists}, \
                 {PROOF_HARNESS_WASM_PATH} exists={proof_exists}"
            ),
        ));
        return;
    }

    check_one_wasm_size_budget(
        root,
        findings,
        PUBLIC_SHOWCASE_WASM_PATH,
        PUBLIC_SHOWCASE_WASM_RAW_BUDGET_BYTES,
        PUBLIC_SHOWCASE_WASM_BROTLI_BUDGET_BYTES,
        "public showcase",
    );
    check_one_wasm_size_budget(
        root,
        findings,
        PROOF_HARNESS_WASM_PATH,
        PROOF_HARNESS_WASM_RAW_BUDGET_BYTES,
        PROOF_HARNESS_WASM_BROTLI_BUDGET_BYTES,
        "proof harness",
    );
}

fn check_one_wasm_size_budget(
    root: &Path,
    findings: &mut Vec<Finding>,
    relative_path: &str,
    raw_budget_bytes: u64,
    brotli_budget_bytes: u64,
    label: &str,
) {
    let path = root.join(relative_path);
    let bytes = match fs::read(&path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "PUBLIC-SHOWCASE-WASM-SIZE",
                format!("could not read {label} WASM bundle {relative_path}: {error}"),
            ));
            return;
        }
    };
    let raw_len = bytes.len() as u64;
    let manifest_path = root.join(format!("{relative_path}.size.json"));
    let manifest = match read_wasm_size_manifest(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "PUBLIC-SHOWCASE-WASM-SIZE",
                format!(
                    "could not read {label} WASM size manifest {}: {error}",
                    path_to_forward_slash(&manifest_path)
                ),
            ));
            return;
        }
    };
    if manifest.raw_bytes != raw_len {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM size manifest raw_bytes {} does not match actual raw size {raw_len} at {relative_path}",
                manifest.raw_bytes
            ),
        ));
    }
    if manifest.brotli_quality != 11 {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM size manifest must use brotli quality 11, got {}",
                manifest.brotli_quality
            ),
        ));
    }
    if raw_len > raw_budget_bytes {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM raw size {raw_len} exceeds budget {raw_budget_bytes} at {relative_path}"
            ),
        ));
    }
    if manifest.brotli_bytes > brotli_budget_bytes {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM brotli size {} exceeds budget {brotli_budget_bytes} at {relative_path}",
                manifest.brotli_bytes
            ),
        ));
    }
}

struct WasmSizeManifest {
    raw_bytes: u64,
    brotli_quality: u64,
    brotli_bytes: u64,
}

fn read_wasm_size_manifest(path: &Path) -> Result<WasmSizeManifest, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let json: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    Ok(WasmSizeManifest {
        raw_bytes: json
            .get("raw_bytes")
            .and_then(Value::as_u64)
            .ok_or("missing numeric raw_bytes")?,
        brotli_quality: json
            .get("brotli_quality")
            .and_then(Value::as_u64)
            .ok_or("missing numeric brotli_quality")?,
        brotli_bytes: json
            .get("brotli_bytes")
            .and_then(Value::as_u64)
            .ok_or("missing numeric brotli_bytes")?,
    })
}
