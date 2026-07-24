use crate::app::prelude::*;

pub(super) fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !value.bytes().all(|byte| byte == b'0')
}

pub(super) fn write_browser_visual_proof(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
    backend: &str,
    lane: &str,
) -> Result<(), String> {
    let results = super::super::stage_artifacts::browser_release_results(files)?;
    let result = super::super::stage_artifacts::validate_browser_backend_result(&results, backend)?;
    let proof = visual_proof_base(lane, expected_commit, "browser-rust-wasm-rendered-output")
        .with_source(
            &output.join("m6-rust-wasm-renderer-probe.json"),
            "m6-rust-wasm-renderer-probe.json",
        )?
        .with_extra(json!({
            "backend": backend,
            "pixel_source": "renderer-owned-gpu-copy",
            "nonblack_pixels": super::super::stage_artifacts::browser_nonblack_pixels(&result),
            "renderer_readback": result.get("renderer_readback").cloned().unwrap_or(Value::Null),
            "screenshot_metadata": result.get("screenshot_metadata").cloned().unwrap_or(Value::Null),
        }))
        .finish();
    super::super::stage_artifacts::write_stage_json(
        &output.join(format!("visual-proof/{lane}.json")),
        &proof,
    )
}

pub(super) fn write_native_gpu_visual_proof(
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    for lane in ["macos-metal", "windows-dx12", "linux-native-vulkan"] {
        let suffix = format!("m9-platform/{lane}/rendered-output.json");
        let path = output.join(&suffix);
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let value = serde_json::from_str::<Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        if native_gpu_render_proof_passes(&value) {
            let proof =
                visual_proof_base("native-gpu", expected_commit, "native-gpu-rendered-output")
                    .with_source(&path, &suffix)?
                    .with_extra(json!({
                        "source_lane": lane,
                        "source_artifact": suffix,
                        "backend": value.get("backend").cloned().unwrap_or(Value::Null),
                        "gpu_proof": true,
                    }))
                    .finish();
            return super::super::stage_artifacts::write_stage_json(
                &output.join("visual-proof/native-gpu.json"),
                &proof,
            );
        }
    }
    Err("no native GPU rendered-output artifact proves GPU output".to_string())
}

pub(super) struct VisualProofBuilder {
    value: Value,
}

pub(super) fn visual_proof_base(
    lane: &str,
    expected_commit: &str,
    proof_class: &str,
) -> VisualProofBuilder {
    VisualProofBuilder {
        value: json!({
            "schema": "scena.visual_proof.v1",
            "producer": "cargo run -p xtask -- stage-release-artifacts",
            "lane": lane,
            "status": "passed",
            "preview_only": false,
            "rust_test_command": false,
            "rust_test_output_observed": false,
            "skip_marker_observed": false,
            "release_evidence": true,
            "proof_class": proof_class,
            "commit_sha": expected_commit,
            "timestamp_unix_seconds": current_unix_seconds(),
        }),
    }
}

impl VisualProofBuilder {
    pub(super) fn with_source(mut self, source: &Path, relative: &str) -> Result<Self, String> {
        self.value["source_artifact_path"] = json!(relative);
        self.value["source_artifact_sha256"] =
            json!(sha256_hex(source).map_err(|error| error.to_string())?);
        Ok(self)
    }

    pub(super) fn with_extra(mut self, extra: Value) -> Self {
        if let (Some(target), Some(extra)) = (self.value.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                target.insert(key.clone(), value.clone());
            }
        }
        self
    }

    pub(super) fn finish(self) -> Value {
        self.value
    }
}
